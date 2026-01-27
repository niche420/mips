pub mod gamepad;
pub mod memory_card;

use log::warn;
use crate::input::{Button, ButtonState};
use crate::ps1::psx::addressable::{AccessWidth, Addressable};
use crate::ps1::psx::bus::Bus;
use crate::ps1::psx::processor::{irq, ClockCycle};
use crate::ps1::psx::processor::irq::Interrupt;
use crate::ps1::psx::sync;

const PADSYNC: sync::SyncToken = sync::SyncToken::PadMemCard;

pub struct Peripheral {
    /// Connected device
    device: Box<dyn DeviceInterface>,
    /// Counter keeping track of the current position in the reply sequence
    seq: u8,
    /// False if the device is done processing the current command
    active: bool,
}

impl Peripheral {
    fn new(device: Box<dyn DeviceInterface>) -> Peripheral {
        Peripheral {
            device,
            seq: 0,
            active: false,
        }
    }

    /// Called when the "select" line goes low.
    pub fn select(&mut self) {
        // Prepare for incoming command
        self.active = true;
        self.seq = 0;

        self.device.select();
    }

    /// The 1st return value is the response byte. The 2nd return value contains the state of the
    /// DSR pulse to notify the controller that more data can be read. If the device wants to
    /// complete the transaction it'll return DsrState::Idle
    pub fn exchange_byte(&mut self, cmd: u8) -> (u8, DsrState) {
        if !self.active {
            return (0xff, DsrState::Idle);
        }

        let (resp, dsr_state) = self.device.handle_command(self.seq, cmd);

        // If we're not asserting DSR it either means that we've encountered an error or that we
        // have nothing else to reply. In either case we won't be handling any more command bytes
        // in this transaction.
        self.active = dsr_state != DsrState::Idle;

        self.seq += 1;

        (resp, dsr_state)
    }

    /// Return a reference to the connected device
    pub fn device(&self) -> &dyn DeviceInterface {
        &*self.device
    }

    /// Return a mutable reference to the connected device
    pub fn device_mut(&mut self) -> &mut dyn DeviceInterface {
        &mut *self.device
    }

    /// Change the connected device, returning the old one (will return an instance of
    /// DisconnectedDevice if there was no previously connected device)
    pub fn connect_device(
        &mut self,
        mut device: Box<dyn DeviceInterface>,
    ) -> Box<dyn DeviceInterface> {
        std::mem::swap(&mut self.device, &mut device);

        self.device.connected();

        device
    }

    /// Disconnect the device and return it. Returns an instance of DisconnectedDevice if nothing
    /// was connected
    pub fn disconnect_device(&mut self) -> Box<dyn DeviceInterface> {
        self.connect_device(Box::new(DisconnectedDevice))
    }
}

struct SerializedPeripheral {
    seq: u8,
    active: bool,
}

/// Trait used to abstract away the various device types.
///
/// This can be used to implement both controllers and memory cards. Obviously the methods that are
/// irrelevant for the concrete device should be left unimplemented (no sense getting the
/// `write_counter` of a DualShock or setting the `axis_state` of a MemoryCard.
pub trait DeviceInterface {
    /// Human-readable description of the device
    fn description(&self) -> String;

    /// Called every time the device is selected (i.e. the "/select" signal goes low)
    fn select(&mut self) {}

    /// Handle a command byte sent by the console. `seq` is the byte position in the current
    /// command starting with `1` since byte `0` is expected to always be `0x01` when addressing a
    /// controller and is handled at the top level.
    ///
    /// Returns a pair `(response, dsr)`. If DSR is false the subsequent command bytes will be
    /// ignored for the current transaction.
    fn handle_command(&mut self, seq: u8, cmd: u8) -> (u8, DsrState);

    /// Set the state of individual buttons
    fn set_button_state(&mut self, _button: Button, _state: ButtonState) {}

    /// Set the state of the axis. Each pair is `(x, y)`.
    fn set_axis_state(&mut self, _left: (i16, i16), _right: (i16, i16)) {}

    /// Get rumble state. The first u8 is the big motor in the left handle, the 2nd is the small
    /// motor in the right handle.
    fn get_rumble(&self) -> (u8, u8) {
        (0, 0)
    }

    /// Dump the entirety of the device's flash (if it exists). Probably only useful for Memory
    /// Cards.
    fn get_memory(&self) -> Option<&[u8; memory_card::FLASH_SIZE]> {
        None
    }

    /// Returns the value of a counter that's incremented every time the memory card's flash is
    /// written (unless the write didn't change the flash contents, in which case it's ignored).
    /// Can be used to check if the contents of the memory card should be written to disk.
    fn write_counter(&self) -> u32 {
        0
    }

    /// Called when the device is connected to a console
    fn connected(&mut self) {}

    /// Called once per frame
    fn new_frame(&mut self) {}
}

/// Dummy profile emulating an empty pad or memory card slot
pub struct DisconnectedDevice;

impl DeviceInterface for DisconnectedDevice {
    fn description(&self) -> String {
        "Disconnected".to_string()
    }

    fn handle_command(&mut self, _: u8, _: u8) -> (u8, DsrState) {
        // The bus is open, no response
        (0xff, DsrState::Idle)
    }
}

pub fn disconnected_gamepad() -> Peripheral {
    Peripheral::new(Box::new(DisconnectedDevice))
}

pub fn disconnected_memory_card() -> Peripheral {
    Peripheral::new(Box::new(DisconnectedDevice))
}

pub struct PadMemCard {
    /// Serial clock divider. The LSB is read/write but is not used, This way the hardware divide
    /// the CPU clock by half of `baud_div` and can invert the serial clock polarity twice every
    /// `baud_div` which effectively means that the resulting frequency is CPU clock / (`baud_div`
    /// & 0xfe).
    baud_div: u16,
    /// Serial config, not implemented for now...
    mode: u8,
    /// Transmission enabled if true
    tx_en: bool,
    /// Pending TX byte, if any
    tx_pending: Option<u8>,
    /// If true the targeted peripheral select signal is asserted (the actual signal is active low,
    /// so it's driving low on the controller port when `select` is true). The `target` field says
    /// which peripheral is addressed.
    select: bool,
    /// This bit says which of the two pad/memorycard port pair we're selecting with `select_n`
    /// above. Multitaps are handled at the serial protocol level, not by dedicated hardware pins.
    target: Target,
    /// Control register bits 3 and 5 are read/write but I don't know what they do. I just same
    /// them here for accurate readback.
    unknown: u8,
    /// XXX not sure what this does exactly, forces a read without any TX?
    rx_en: bool,
    /// If true an interrupt is generated when a DSR pulse is received from the pad/memory card
    dsr_it: bool,
    /// Current interrupt level
    interrupt: bool,
    /// Current response byte.
    /// XXX Normally it should be a FIFO but I'm not sure how it works really. Besides the game
    /// should check for the response after each byte anyway, so it's probably unused the vast
    /// majority of times.
    response: u8,
    /// True when we the RX FIFO is not empty.
    rx_not_empty: bool,
    /// Gamepad in slot 1
    pad1: Peripheral,
    pad1_dsr: DsrState,
    /// Gamepad in slot 2
    pad2: Peripheral,
    pad2_dsr: DsrState,
    /// Memory Card in slot 1
    memcard1: Peripheral,
    memcard1_dsr: DsrState,
    /// Memory Card in slot 2
    memcard2: Peripheral,
    memcard2_dsr: DsrState,
    /// Bus state machine
    transfer_state: TransferState,
}

impl PadMemCard {
    pub fn new() -> PadMemCard {
        PadMemCard {
            baud_div: 0,
            mode: 0,
            tx_en: false,
            tx_pending: None,
            select: false,
            target: Target::PadMemCard1,
            interrupt: false,
            unknown: 0,
            rx_en: false,
            dsr_it: false,
            response: 0xff,
            rx_not_empty: false,
            pad1: disconnected_gamepad(),
            pad1_dsr: DsrState::Idle,
            pad2: disconnected_gamepad(),
            pad2_dsr: DsrState::Idle,
            memcard1: disconnected_memory_card(),
            memcard1_dsr: DsrState::Idle,
            memcard2: disconnected_memory_card(),
            memcard2_dsr: DsrState::Idle,
            transfer_state: TransferState::Idle,
        }
    }

    /// Return a mutable reference to the gamepad peripherals being used.
    pub fn gamepads_mut(&mut self) -> [&mut Peripheral; 2] {
        [&mut self.pad1, &mut self.pad2]
    }

    /// Return a reference to the memory card peripherals being used.
    pub fn memory_cards(&self) -> [&Peripheral; 2] {
        [&self.memcard1, &self.memcard2]
    }

    /// Return a mutable reference to the memory card peripherals being used.
    pub fn memory_cards_mut(&mut self) -> [&mut Peripheral; 2] {
        [&mut self.memcard1, &mut self.memcard2]
    }

    fn maybe_exchange_byte(&mut self) {
        let to_send = match self.tx_pending {
            Some(b) => b,
            None => return,
        };

        if !self.tx_en {
            // Nothing to do
            return;
        }

        if !self.transfer_state.is_idle() {
            // I'm guessing that we wait for the current command to be over before we send the next
            // one?
            return;
        }

        if self.baud_div < 80 || self.baud_div > 239 {
            // XXX Controller timings are tricky to get absolutely right. The code below is fairly
            // accurate for values between 80 and 239. Before and after that there's a "gap". See:
            // https://svkt.org/~simias/up/20200410-000241_pad_controller_timings.dat.png
            //
            // Fortunately almost all games seem to use a baud rate of 0x88 (136). If some games
            // use a different value (maybe with some exotic peripherals?) it'll probably be worth
            // reviewing this
            unimplemented!("Baud divider {}", self.baud_div);
        }

        if !self.select {
            // In this situation in my tests the following happens:
            //
            // * The "TxStart" phase works as usual (i.e. the bit goes up after ~baud_div cycles)
            // * The transfer never finishes. RX not empty never goes up.
            // * Setting the "select" bit after TxStart (in an effort to unfreeze the transfer)
            //   doesn't seem to do anything.
            unimplemented!("Pad/MemCard TX without selection");
        }

        self.tx_pending = None;

        let bd = ClockCycle::from(self.baud_div);
        // This value varies quite a bit on the real hardware, probably depending on the current
        // value of the divider's counter or something like that?
        //
        // With the divider set at 136 I see it go as low as 67 and as high as 207
        let to_tx_start = bd - 40;
        let tx_total = (bd - 11) * 11;
        let to_tx_end = tx_total - to_tx_start;

        // This is the moment at which the controller seems to actually process the command. This
        // occurs about `baud_divider` cycles before RX not empty goes up
        let to_dsr_start = tx_total - bd;

        // I suppose that it would be more accurate to call this code at the end of the transfer
        // since it's at this point that the controller can really process the command, but it
        // shouldn't make much of a difference for most peripherals. It could add a bit more input
        // lag if we're very unlucky and the transfer occurs during a frame boundary but given that
        // with the standard baudrate of 136 a transfer takes about 40 us it's very unlikely.
        let response = match self.target {
            Target::PadMemCard1 => {
                let (pad_response, pad_dsr_state) = self.pad1.exchange_byte(to_send);
                let (mc_response, mc_dsr_state) = self.memcard1.exchange_byte(to_send);

                self.pad1_dsr = pad_dsr_state.delay_by(to_dsr_start);
                self.memcard1_dsr = mc_dsr_state.delay_by(to_dsr_start);

                pad_response & mc_response
            }
            Target::PadMemCard2 => {
                let (pad_response, pad_dsr_state) = self.pad2.exchange_byte(to_send);
                let (mc_response, mc_dsr_state) = self.memcard2.exchange_byte(to_send);

                self.pad2_dsr = pad_dsr_state.delay_by(to_dsr_start);
                self.memcard2_dsr = mc_dsr_state.delay_by(to_dsr_start);

                pad_response & mc_response
            }
        };

        self.transfer_state = TransferState::TxStart(to_tx_start, to_tx_end, response);
    }

    /// Returns true if any of the device's DSR is active
    fn dsr_active(&self) -> bool {
        self.pad1_dsr.is_active()
            || self.pad2_dsr.is_active()
            || self.memcard1_dsr.is_active()
            || self.memcard2_dsr.is_active()
    }

    fn get_response(&mut self) -> u8 {
        let res = self.response;

        self.rx_not_empty = false;
        self.response = 0xff;

        res
    }

    fn stat(&self) -> u32 {
        let mut stat = 0u32;

        // In my tests this bit *only* does down during between the moment we write a byte in the
        // TX buffer and the end of the TxStart step. The rest of the time it stays up
        let tx_ready = if let TransferState::TxStart(_, _, _) = self.transfer_state {
            false
        } else {
            self.tx_pending.is_none()
        };

        stat |= tx_ready as u32;
        stat |= (self.rx_not_empty as u32) << 1;
        // TX Ready flag 2 (XXX what's that about?)
        stat |= 1 << 2;
        // RX parity error should always be 0 in our case.
        stat |= 0 << 3;
        stat |= (self.dsr_active() as u32) << 7;
        stat |= (self.interrupt as u32) << 9;
        // XXX needs to add the baudrate counter in bits [31:11];
        stat |= 0 << 11;

        stat
    }

    fn set_mode(&mut self, mode: u8) {
        if mode == self.mode {
            return;
        }

        if !self.transfer_state.is_idle() {
            warn!("Pad/Memcard controller mode change while transfer is taking place");
        }

        self.mode = mode;
    }

    fn control(&self) -> u16 {
        let mut ctrl = 0u16;

        ctrl |= self.unknown as u16;

        ctrl |= self.tx_en as u16;
        ctrl |= (self.select as u16) << 1;
        ctrl |= (self.rx_en as u16) << 2;
        // XXX Add other interrupts when they're implemented
        ctrl |= (self.dsr_it as u16) << 12;
        ctrl |= (self.target as u16) << 13;

        ctrl
    }

    /// Returns `true` if an interrupt should be triggered
    fn set_control(&mut self, ctrl: u16) {
        let prev_select = self.select;
        let prev_target = self.target;

        if ctrl & 0x40 != 0 {
            // Soft reset
            // XXX It doesn't seem to reset the contents of the RX FIFO, needs more testing
            self.baud_div = 0;
            self.mode = 0;
            self.select = false;
            self.target = Target::PadMemCard1;
            self.unknown = 0;
            self.interrupt = false;
            self.rx_not_empty = false;
            self.transfer_state = TransferState::Idle;
        } else {
            if ctrl & 0x10 != 0 {
                // Interrupt acknowledge
                self.interrupt = false;
            }

            // No idea what bits 3 and 5 do but they're read/write.
            self.unknown = (ctrl as u8) & 0x28;

            self.tx_en = ctrl & 1 != 0;
            self.select = (ctrl >> 1) & 1 != 0;
            self.rx_en = (ctrl >> 2) & 1 != 0;
            self.dsr_it = (ctrl >> 12) & 1 != 0;
            self.target = Target::from_control(ctrl);

            if self.rx_en {
                panic!("Gamepad rx_en not implemented");
            }

            if !self.interrupt {
                self.refresh_irq_level();
                if self.interrupt {
                    // Interrupt should trigger here but that really shouldn't happen I think.
                    panic!("dsr_it enabled while DSR signal is active");
                }
            }

            if ctrl & 0xf00 != 0 {
                // XXX add support for those interrupts
                panic!("Unsupported gamepad interrupts: {:04x}", ctrl);
            }
        }

        // If the select line was just asserted or we changed the active line we need to notify the
        // devices since this means that a fresh transaction is about to start.
        if self.select && (!prev_select || self.target != prev_target) {
            match self.target {
                Target::PadMemCard1 => {
                    self.pad1.select();
                    self.memcard1.select()
                }
                Target::PadMemCard2 => {
                    self.pad2.select();
                    self.memcard2.select();
                }
            }
        }

        if !self.select || self.target != Target::PadMemCard1 {
            // Unselected pads/memcards don't send DSR
            self.pad1_dsr = DsrState::Idle;
            self.memcard1_dsr = DsrState::Idle;
        }

        if !self.select || self.target != Target::PadMemCard2 {
            // Unselected pads/memcards don't send DSR
            self.pad2_dsr = DsrState::Idle;
            self.memcard2_dsr = DsrState::Idle;
        }

        let prev_interrupt = self.interrupt;
        self.refresh_irq_level();

        if !prev_interrupt && self.interrupt {
            // The controller's "dsr_it" interrupt is not edge triggered: as long as self.dsr &&
            // self.dsr_it is true it will keep being triggered. If the software attempts to
            // acknowledge the interrupt in this state it will re-trigger immediately which will be
            // seen by the edge-triggered top level interrupt controller. So I guess this shouldn't
            // happen?
            warn!("Gamepad interrupt acknowledge while DSR is active");
        }
    }

    fn refresh_irq_level(&mut self) {
        self.interrupt |= self.dsr_active() && self.dsr_it;
    }
}

fn run_controller(bus: &mut Bus) {
    let elapsed = sync::resync(bus, PADSYNC);

    run_transfer(bus, elapsed);
    run_dsr(bus, elapsed);
}

/// Update transfer state machine
fn run_transfer(bus: &mut Bus, mut cycles: ClockCycle) {
    while cycles > 0 {
        let elapsed = match bus.pad_memcard.transfer_state {
            TransferState::Idle => cycles,
            TransferState::TxStart(delay, to_rx, rx_byte) => {
                if cycles < delay {
                    bus.pad_memcard.transfer_state =
                        TransferState::TxStart(delay - cycles, to_rx, rx_byte);

                    cycles
                } else {
                    bus.pad_memcard.transfer_state = TransferState::RxAvailable(to_rx, rx_byte);

                    delay
                }
            }
            TransferState::RxAvailable(delay, rx_byte) => {
                if cycles < delay {
                    bus.pad_memcard.transfer_state =
                        TransferState::RxAvailable(delay - cycles, rx_byte);

                    cycles
                } else {
                    if bus.pad_memcard.rx_not_empty {
                        // XXX should push in the non-emulated RX FIFO instead of overwriting
                        // `psx.pad_memcard.response`
                        unimplemented!("Gamepad RX while FIFO isn't empty");
                    }

                    bus.pad_memcard.response = rx_byte;
                    bus.pad_memcard.rx_not_empty = true;
                    bus.pad_memcard.transfer_state = TransferState::Idle;

                    delay
                }
            }
        };

        // Need to call this here if we have a buffered transfer. That normally shouldn't happen
        // since the game should wait for the DSR pulse first
        bus.pad_memcard.maybe_exchange_byte();

        cycles -= elapsed;
    }
}

/// Update the device's DSR state
fn run_dsr(bus: &mut Bus, cycles: ClockCycle) {
    bus.pad_memcard.pad1_dsr.run(cycles);
    bus.pad_memcard.pad2_dsr.run(cycles);
    bus.pad_memcard.memcard1_dsr.run(cycles);
    bus.pad_memcard.memcard2_dsr.run(cycles);

    // See if a new DSR pulse occurred to trigger the IRQ
    bus.pad_memcard.refresh_irq_level();
    irq::set_level(bus, Interrupt::PadMemCard, bus.pad_memcard.interrupt);
}

fn predict_next_sync(bus: &mut Bus) {
    let mut next_event = 1_000_000;

    if bus.pad_memcard.dsr_it {
        if let Some(e) = bus.pad_memcard.pad1_dsr.to_dsr() {
            if e < next_event {
                next_event = e;
            }
        }
        if let Some(e) = bus.pad_memcard.pad2_dsr.to_dsr() {
            if e < next_event {
                next_event = e;
            }
        }
        if let Some(e) = bus.pad_memcard.memcard1_dsr.to_dsr() {
            if e < next_event {
                next_event = e;
            }
        }
        if let Some(e) = bus.pad_memcard.memcard2_dsr.to_dsr() {
            if e < next_event {
                next_event = e;
            }
        }
    }

    sync::next_event(bus, PADSYNC, next_event);
}

pub fn run(bus: &mut Bus) {
    run_controller(bus);
    predict_next_sync(bus);
}

pub fn store<T: Addressable>(bus: &mut Bus, off: u32, val: T) {
    run_controller(bus);

    let v = val.as_u16();

    match off {
        0 => {
            if T::width() != AccessWidth::Byte {
                unimplemented!("Gamepad TX access ({:?})", T::width());
            }

            if bus.pad_memcard.tx_pending.is_some() {
                warn!("Dropping pad/memcard byte before send");
            }

            bus.pad_memcard.tx_pending = Some(v as u8);
        }
        8 => bus.pad_memcard.set_mode(val.as_u8()),
        10 => {
            if T::width() == AccessWidth::Byte {
                // Byte access behaves like a halfword
                unimplemented!("Unhandled byte gamepad control access");
            }
            bus.pad_memcard.set_control(v);
            irq::set_level(bus, Interrupt::PadMemCard, bus.pad_memcard.interrupt);
        }
        14 => bus.pad_memcard.baud_div = v,
        _ => warn!("Write to gamepad register {} {:04x}", off, v),
    }

    bus.pad_memcard.maybe_exchange_byte();

    predict_next_sync(bus);
}

pub fn load<T: Addressable>(bus: &mut Bus, off: u32) -> T {
    run_controller(bus);

    let v = match off {
        0 => {
            if T::width() != AccessWidth::Byte {
                unimplemented!("Unhandled gamepad RX access ({:?})", T::width());
            }

            u32::from(bus.pad_memcard.get_response())
        }
        4 => bus.pad_memcard.stat(),
        8 => u32::from(bus.pad_memcard.mode),
        10 => u32::from(bus.pad_memcard.control()),
        14 => u32::from(bus.pad_memcard.baud_div),
        _ => {
            warn!("pad_memcard read {:?} 0x{:x}", T::width(), off);
            0
        }
    };

    predict_next_sync(bus);

    T::from_u32(v)
}

/// Identifies the target of the serial communication, either the gamepad/memory card port 0 or 1.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    PadMemCard1 = 0,
    PadMemCard2 = 1,
}

impl Target {
    fn from_control(ctrl: u16) -> Target {
        if ctrl & 0x2000 == 0 {
            Target::PadMemCard1
        } else {
            Target::PadMemCard2
        }
    }
}

/// Controller transaction state machine
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
enum TransferState {
    /// Bus is idle
    Idle,
    /// We just started a new transfer. This is the delay until the "TX started" (stat bit 0) goes
    /// up. The 2nd value is the subsequent RxAvailable delay.
    TxStart(ClockCycle, ClockCycle, u8),
    /// Transfer is in progress. This is the delay until the data is put in the RX FIFO and the "RX
    /// not empty" (stat bit 1) goes up.
    RxAvailable(ClockCycle, u8),
}

impl TransferState {
    fn is_idle(&self) -> bool {
        *self == TransferState::Idle
    }
}

/// State of the DSR (data available) signal coming from one of the pads or memcards
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
pub enum DsrState {
    /// No event pending
    Idle,
    /// A DSR pulse is about to take place. The first value is the number of cycles until the start
    /// of the pulse, the 2nd is the length of the pulse.
    Pending(ClockCycle, ClockCycle),
    /// A DSR pulse is taking place. The value is the remaining number of cycles until the end of
    /// the pulse.
    Active(ClockCycle),
}

impl DsrState {
    fn is_active(&self) -> bool {
        matches!(self, DsrState::Active(_))
    }

    fn delay_by(&self, offset: ClockCycle) -> DsrState {
        match *self {
            DsrState::Idle => DsrState::Idle,
            DsrState::Pending(delay, duration) => DsrState::Pending(delay + offset, duration),
            DsrState::Active(_) => unreachable!("Can't delay an active pulse!"),
        }
    }

    /// Returns the number of cycles until the DSR pulse, if one is pending
    fn to_dsr(&self) -> Option<ClockCycle> {
        match *self {
            DsrState::Idle => None,
            DsrState::Pending(delay, _) => Some(delay),
            DsrState::Active(_) => None,
        }
    }

    fn run(&mut self, mut cycles: ClockCycle) {
        while cycles > 0 {
            *self = match *self {
                DsrState::Idle => {
                    cycles = 0;
                    DsrState::Idle
                }
                DsrState::Pending(delay, duration) => {
                    if delay > cycles {
                        let rem = delay - cycles;
                        cycles = 0;
                        DsrState::Pending(rem, duration)
                    } else {
                        cycles -= delay;

                        DsrState::Active(duration)
                    }
                }
                DsrState::Active(duration) => {
                    if duration > cycles {
                        let rem = duration - cycles;
                        cycles = 0;
                        DsrState::Active(rem)
                    } else {
                        cycles -= duration;

                        DsrState::Idle
                    }
                }
            };
        }
    }
}

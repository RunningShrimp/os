//! Timer Driver - PIT and HPET timer initialization and management
//!
//! Provides:
//! - 8254 PIT (Programmable Interval Timer) configuration
//! - HPET (High Precision Event Timer) support
//! - Boot-time delay and interval measurement
//! - Interrupt-driven timer management

/// PIT base I/O address
pub const PIT_BASE: u16 = 0x40;

/// PIT register ports
pub const PIT_CHANNEL_0: u16 = 0x40;  // Counter 0
pub const PIT_CHANNEL_1: u16 = 0x41;  // Counter 1
pub const PIT_CHANNEL_2: u16 = 0x42;  // Counter 2
pub const PIT_CONTROL: u16 = 0x43;    // Control word

/// PIT clock frequency (1.193182 MHz)
pub const PIT_CLOCK_HZ: u32 = 1193182;

/// HPET base address (typically 0xFED00000)
pub const HPET_BASE: u64 = 0xFED00000;

/// PIT operating modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitMode {
    /// Mode 0: Interrupt on terminal count
    InterruptOnTerminal = 0,
    /// Mode 1: Hardware-triggered one-shot
    HardwareOneShot = 1,
    /// Mode 2: Rate generator
    RateGenerator = 2,
    /// Mode 3: Square wave generator
    SquareWave = 3,
    /// Mode 4: Software-triggered strobe
    SoftwareStrobe = 4,
    /// Mode 5: Hardware-triggered strobe
    HardwareStrobe = 5,
}

/// PIT counter selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitCounter {
    /// Counter 0 (system timer)
    Counter0 = 0,
    /// Counter 1 (DRAM refresh)
    Counter1 = 1,
    /// Counter 2 (speaker tone)
    Counter2 = 2,
}

/// Access mode for PIT counter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Latch count
    LatchCount = 0,
    /// Low byte only
    LowByteOnly = 1,
    /// High byte only
    HighByteOnly = 2,
    /// Both bytes
    BothBytes = 3,
}

/// PIT control word builder
#[derive(Debug, Clone, Copy)]
pub struct PitControl {
    /// Counter selection
    pub counter: PitCounter,
    /// Access mode
    pub access_mode: AccessMode,
    /// Operating mode
    pub mode: PitMode,
    /// Binary or BCD counting
    pub binary_mode: bool,
}

impl PitControl {
    /// Create PIT control word
    pub fn new(
        counter: PitCounter,
        access_mode: AccessMode,
        mode: PitMode,
    ) -> Self {
        PitControl {
            counter,
            access_mode,
            mode,
            binary_mode: true,
        }
    }

    /// Encode to control register value
    pub fn encode(&self) -> u8 {
        let mut value = 0u8;
        value |= ((self.counter as u8) & 0x3) << 6;
        value |= ((self.access_mode as u8) & 0x3) << 4;
        value |= ((self.mode as u8) & 0x7) << 1;
        if !self.binary_mode {
            value |= 0x01;
        }
        value
    }
}

/// HPET (High Precision Event Timer) configuration
#[derive(Debug, Clone, Copy)]
pub struct HpetConfig {
    /// HPET base address
    pub base_address: u64,
    /// Main counter frequency (Hz)
    pub main_counter_freq: u32,
    /// Number of timers
    pub timer_count: u8,
    /// Is HPET enabled
    pub enabled: bool,
}

impl HpetConfig {
    /// Create HPET configuration
    pub fn new(base_address: u64) -> Self {
        HpetConfig {
            base_address,
            main_counter_freq: 0,
            timer_count: 0,
            enabled: false,
        }
    }
}

/// Timer tick information
#[derive(Debug, Clone, Copy)]
pub struct TimerTick {
    /// Tick number
    pub tick: u64,
    /// Nanoseconds per tick
    pub ns_per_tick: u32,
}

/// PIT timer controller
pub struct PitTimer {
    /// Base I/O address
    base_address: u16,
    /// Current mode
    current_mode: PitMode,
    /// Current frequency (Hz)
    frequency: u32,
    /// Counter reload value
    reload_value: u16,
    /// Number of ticks
    tick_count: u64,
    /// Initialized flag
    initialized: bool,
}

impl PitTimer {
    /// Create PIT timer instance
    pub fn new() -> Self {
        PitTimer {
            base_address: PIT_BASE,
            current_mode: PitMode::RateGenerator,
            frequency: 1000,
            reload_value: 0,
            tick_count: 0,
            initialized: false,
        }
    }

    /// Initialize PIT with frequency
    pub fn initialize(&mut self, frequency: u32) -> bool {
        if frequency == 0 || frequency > PIT_CLOCK_HZ {
            return false;
        }

        self.frequency = frequency;
        self.reload_value = (PIT_CLOCK_HZ / frequency) as u16;

        // Configure counter 0 for rate generator mode
        let control = PitControl::new(
            PitCounter::Counter0,
            AccessMode::BothBytes,
            PitMode::RateGenerator,
        );

        self.write_control(control.encode());

        // Set reload value (low byte first, then high byte)
        self.write_counter(PIT_CHANNEL_0, (self.reload_value & 0xFF) as u8);
        self.write_counter(PIT_CHANNEL_0, ((self.reload_value >> 8) & 0xFF) as u8);

        self.initialized = true;
        true
    }

    /// Set operating mode
    pub fn set_mode(&mut self, mode: PitMode) -> bool {
        if !self.initialized {
            return false;
        }

        self.current_mode = mode;
        let control = PitControl::new(
            PitCounter::Counter0,
            AccessMode::BothBytes,
            mode,
        );

        self.write_control(control.encode());
        true
    }

    /// Change frequency
    pub fn set_frequency(&mut self, frequency: u32) -> bool {
        if !self.initialized || frequency == 0 || frequency > PIT_CLOCK_HZ {
            return false;
        }

        self.frequency = frequency;
        self.reload_value = (PIT_CLOCK_HZ / frequency) as u16;

        // Write new reload value
        self.write_counter(PIT_CHANNEL_0, (self.reload_value & 0xFF) as u8);
        self.write_counter(PIT_CHANNEL_0, ((self.reload_value >> 8) & 0xFF) as u8);

        true
    }

    /// Record tick occurrence
    pub fn on_tick(&mut self) {
        self.tick_count += 1;
    }

    /// Get tick count
    pub fn get_tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get elapsed milliseconds
    pub fn get_elapsed_ms(&self) -> u64 {
        if self.frequency == 0 {
            return 0;
        }
        (self.tick_count * 1000) / (self.frequency as u64)
    }

    /// Delay in milliseconds (busy wait)
    pub fn delay_ms(&self, ms: u32) {
        let ticks_needed = ((ms as u64) * (self.frequency as u64)) / 1000;
        let start = self.tick_count;
        while self.tick_count - start < ticks_needed {
            // Add compiler fence to prevent optimization that might cache self.tick_count
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Get timer report
    pub fn timer_report(&self) -> TimerReport {
        TimerReport {
            initialized: self.initialized,
            frequency: self.frequency,
            tick_count: self.tick_count,
            elapsed_ms: self.get_elapsed_ms(),
            mode: self.current_mode,
        }
    }

    /// Write to control port
    fn write_control(&self, _value: u8) {
        // Real implementation uses outb(value, PIT_CONTROL)
    }

    /// Write to counter port
    fn write_counter(&self, _port: u16, _value: u8) {
        // Real implementation uses outb(value, port)
    }

    /// Read from counter port
    #[allow(dead_code)]
    fn read_counter(&self, _port: u16) -> u8 {
        // Real implementation uses inb(port)
        0
    }
}

/// HPET timer controller
pub struct HpetTimer {
    /// HPET configuration
    config: HpetConfig,
    /// Main counter value
    main_counter: u64,
    /// Number of ticks
    tick_count: u64,
}

impl HpetTimer {
    /// Create HPET timer instance
    pub fn new() -> Self {
        HpetTimer {
            config: HpetConfig::new(HPET_BASE),
            main_counter: 0,
            tick_count: 0,
        }
    }

    /// Initialize HPET
    pub fn initialize(&mut self) -> bool {
        self.config.enabled = true;
        self.config.main_counter_freq = 14318180; // Typical HPET frequency
        self.config.timer_count = 8; // Typical timer count
        true
    }

    /// Record tick
    pub fn on_tick(&mut self) {
        self.tick_count += 1;
    }

    /// Get tick count
    pub fn get_tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get main counter
    pub fn get_main_counter(&self) -> u64 {
        self.main_counter
    }

    /// Set main counter value
    pub fn set_main_counter(&mut self, value: u64) {
        self.main_counter = value;
    }

    /// Get HPET report
    pub fn hpet_report(&self) -> HpetReport {
        HpetReport {
            enabled: self.config.enabled,
            frequency: self.config.main_counter_freq,
            timer_count: self.config.timer_count,
            tick_count: self.tick_count,
            main_counter: self.main_counter,
        }
    }
}

/// Timer statistics report
#[derive(Debug, Clone, Copy)]
pub struct TimerReport {
    /// Initialization status
    pub initialized: bool,
    /// Frequency in Hz
    pub frequency: u32,
    /// Number of ticks
    pub tick_count: u64,
    /// Elapsed time in milliseconds
    pub elapsed_ms: u64,
    /// Current mode
    pub mode: PitMode,
}

/// HPET timer report
#[derive(Debug, Clone, Copy)]
pub struct HpetReport {
    /// HPET enabled
    pub enabled: bool,
    /// Frequency in Hz
    pub frequency: u32,
    /// Number of timers
    pub timer_count: u8,
    /// Tick count
    pub tick_count: u64,
    /// Main counter value
    pub main_counter: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pit_clock_frequency() {
        assert_eq!(PIT_CLOCK_HZ, 1193182);
    }

    #[test]
    fn test_pit_modes() {
        assert_eq!(PitMode::InterruptOnTerminal as u8, 0);
        assert_eq!(PitMode::RateGenerator as u8, 2);
        assert_eq!(PitMode::SquareWave as u8, 3);
    }

    #[test]
    fn test_pit_counters() {
        assert_eq!(PitCounter::Counter0 as u8, 0);
        assert_eq!(PitCounter::Counter1 as u8, 1);
        assert_eq!(PitCounter::Counter2 as u8, 2);
    }

    #[test]
    fn test_access_modes() {
        assert_eq!(AccessMode::LatchCount as u8, 0);
        assert_eq!(AccessMode::BothBytes as u8, 3);
    }

    #[test]
    fn test_pit_control_creation() {
        let ctrl = PitControl::new(
            PitCounter::Counter0,
            AccessMode::BothBytes,
            PitMode::RateGenerator,
        );
        assert_eq!(ctrl.counter, PitCounter::Counter0);
        assert!(ctrl.binary_mode);
    }

    #[test]
    fn test_pit_control_encode() {
        let ctrl = PitControl::new(
            PitCounter::Counter0,
            AccessMode::BothBytes,
            PitMode::RateGenerator,
        );
        let encoded = ctrl.encode();
        assert_eq!(encoded & 0xC0, 0x00); // Counter 0
        assert_eq!(encoded & 0x30, 0x30); // Both bytes
        assert_eq!(encoded & 0x0E, 0x04); // Mode 2
    }

    #[test]
    fn test_pit_timer_creation() {
        let timer = PitTimer::new();
        assert!(!timer.initialized);
        assert_eq!(timer.frequency, 1000);
    }

    #[test]
    fn test_pit_timer_initialize() {
        let mut timer = PitTimer::new();
        assert!(timer.initialize(1000));
        assert!(timer.initialized);
        assert_eq!(timer.frequency, 1000);
    }

    #[test]
    fn test_pit_timer_invalid_frequency() {
        let mut timer = PitTimer::new();
        assert!(!timer.initialize(0));
        assert!(!timer.initialize(PIT_CLOCK_HZ + 1));
    }

    #[test]
    fn test_pit_timer_set_mode() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        assert!(timer.set_mode(PitMode::SquareWave));
        assert_eq!(timer.current_mode, PitMode::SquareWave);
    }

    #[test]
    fn test_pit_timer_set_frequency() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        assert!(timer.set_frequency(2000));
        assert_eq!(timer.frequency, 2000);
    }

    #[test]
    fn test_pit_timer_tick_count() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        assert_eq!(timer.get_tick_count(), 0);
        
        timer.on_tick();
        timer.on_tick();
        assert_eq!(timer.get_tick_count(), 2);
    }

    #[test]
    fn test_pit_timer_elapsed_ms() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        
        // 1000 ticks at 1000 Hz = 1000 ms
        for _ in 0..1000 {
            timer.on_tick();
        }
        assert_eq!(timer.get_elapsed_ms(), 1000);
    }

    #[test]
    fn test_pit_timer_report() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        timer.on_tick();
        
        let report = timer.timer_report();
        assert!(report.initialized);
        assert_eq!(report.frequency, 1000);
        assert_eq!(report.tick_count, 1);
    }

    #[test]
    fn test_hpet_config_creation() {
        let config = HpetConfig::new(HPET_BASE);
        assert_eq!(config.base_address, HPET_BASE);
        assert!(!config.enabled);
    }

    #[test]
    fn test_hpet_timer_creation() {
        let timer = HpetTimer::new();
        assert!(!timer.config.enabled);
    }

    #[test]
    fn test_hpet_timer_initialize() {
        let mut timer = HpetTimer::new();
        assert!(timer.initialize());
        assert!(timer.config.enabled);
        assert_eq!(timer.config.timer_count, 8);
    }

    #[test]
    fn test_hpet_timer_counter() {
        let mut timer = HpetTimer::new();
        timer.initialize();
        
        assert_eq!(timer.get_main_counter(), 0);
        timer.set_main_counter(0x1000);
        assert_eq!(timer.get_main_counter(), 0x1000);
    }

    #[test]
    fn test_hpet_timer_tick() {
        let mut timer = HpetTimer::new();
        timer.initialize();
        
        assert_eq!(timer.get_tick_count(), 0);
        timer.on_tick();
        timer.on_tick();
        assert_eq!(timer.get_tick_count(), 2);
    }

    #[test]
    fn test_hpet_timer_report() {
        let mut timer = HpetTimer::new();
        timer.initialize();
        timer.on_tick();
        
        let report = timer.hpet_report();
        assert!(report.enabled);
        assert_eq!(report.tick_count, 1);
        assert_eq!(report.timer_count, 8);
    }

    #[test]
    fn test_pit_reload_value_calculation() {
        let mut timer = PitTimer::new();
        timer.initialize(1000);
        
        // For 1000 Hz: reload = 1193182 / 1000 = 1193
        assert_eq!(timer.reload_value, 1193);
    }

    #[test]
    fn test_pit_timer_different_frequencies() {
        let mut timer = PitTimer::new();
        
        assert!(timer.initialize(100));
        assert_eq!(timer.reload_value, PIT_CLOCK_HZ / 100);
        
        assert!(timer.set_frequency(1000));
        assert_eq!(timer.reload_value, PIT_CLOCK_HZ / 1000);
    }

    #[test]
    fn test_hpet_base_address() {
        assert_eq!(HPET_BASE, 0xFED00000);
    }

    #[test]
    fn test_pit_ports() {
        assert_eq!(PIT_CHANNEL_0, 0x40);
        assert_eq!(PIT_CHANNEL_1, 0x41);
        assert_eq!(PIT_CHANNEL_2, 0x42);
        assert_eq!(PIT_CONTROL, 0x43);
    }

    #[test]
    fn test_pit_elapsed_time_zero() {
        let timer = PitTimer::new();
        assert_eq!(timer.get_elapsed_ms(), 0);
    }

    #[test]
    fn test_pit_control_bcd_mode() {
        let mut ctrl = PitControl::new(
            PitCounter::Counter0,
            AccessMode::BothBytes,
            PitMode::RateGenerator,
        );
        ctrl.binary_mode = false;
        assert_eq!(ctrl.encode() & 0x01, 0x01);
    }
}

//! CPU Initialization - Multiprocessor, modes, power, interrupts (P1, P8)

pub mod acpi_power_domains;
pub mod dvfs_scaling;
pub mod exception_handler;
pub mod hw_init;
pub mod hypervisor_init;
pub mod idt_manager;
pub mod interrupt_routing;
pub mod interrupts;
pub mod mode_transition;
pub mod multiprocessor_init;
pub mod realmode_switcher;
pub mod sleep_wake_handler;
pub mod virtual_machine;
pub mod virtualization_detect;

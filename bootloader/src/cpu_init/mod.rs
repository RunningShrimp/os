//! CPU Initialization - Multiprocessor, modes, power, interrupts (P1, P8)

pub mod multiprocessor_init;
pub mod mode_transition;
pub mod realmode_switcher;
pub mod acpi_power_domains;
pub mod dvfs_scaling;
pub mod sleep_wake_handler;
pub mod idt_manager;
pub mod exception_handler;
pub mod interrupt_routing;
pub mod interrupts;
pub mod hw_init;
pub mod hypervisor_init;
pub mod virtualization_detect;
pub mod virtual_machine;

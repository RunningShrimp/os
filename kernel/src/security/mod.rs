pub mod acl;
pub mod aslr;
pub mod audit;
pub mod capabilities;
pub mod permission_check;
pub mod seccomp;
pub mod selinux;
pub mod smap_smep;

pub fn init_security_subsystem() -> Result<(), &'static str> {
    Ok(())
}

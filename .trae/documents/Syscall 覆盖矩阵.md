# 系统调用覆盖矩阵

来源: kernel/src/syscall.rs, kernel/src/syscalls.rs

| 编号 | 名称 | 实现函数 | 备注 |
|---:|---|---|---|
| 1 | Fork | sys_fork | 已实现 |
| 2 | Exit | sys_exit | 已实现 |
| 3 | Wait | sys_wait | 已实现 |
| 4 | Pipe | sys_pipe | 已实现 |
| 5 | Read | sys_read | 已实现 |
| 6 | Kill | sys_kill | 已实现 |
| 7 | Exec | sys_exec | 已实现 |
| 8 | Fstat | sys_fstat | 已实现 |
| 9 | Chdir | sys_chdir | 已实现 |
| 10 | Dup | sys_dup | 已实现 |
| 11 | Getpid | sys_getpid | 已实现 |
| 12 | Sbrk | sys_sbrk | 已实现 |
| 13 | Sleep | sys_sleep | 已实现 |
| 14 | Uptime | sys_uptime | 已实现 |
| 15 | Open | sys_open | 已实现 |
| 16 | Write | sys_write | 已实现 |
| 17 | Mknod | sys_mknod | 已实现 |
| 18 | Unlink | sys_unlink | 已实现 |
| 19 | Link | sys_link | 已实现 |
| 20 | Mkdir | sys_mkdir | 已实现 |
| 21 | Close | sys_close | 已实现 |
| 22 | Fcntl | sys_fcntl | 已实现 |
| 23 | Poll | sys_poll | 已实现 |
| 24 | Select | sys_select | 已实现 |
| 25 | Lseek | sys_lseek | 已实现 |
| 26 | Dup2 | sys_dup2 | 已实现 |
| 27 | Getcwd | sys_getcwd | 已实现 |
| 28 | Rmdir | sys_rmdir | 已实现 |
| 44 | Execve | sys_execve | 已实现 |
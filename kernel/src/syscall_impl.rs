
fn sys_lseek(fd: i32, offset: i64, whence: i32) -> isize {
    if fd < 0 { return E_BADF; }
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    let mut table = FILE_TABLE.lock();
    let file = match table.get_mut(file_idx) { Some(f) => f, None => return E_BADF };
    
    if file.ftype == FileType::Pipe {
        return E_PIPE;
    }
    
    let current_size = match file.ftype {
        FileType::Vfs => {
            if let Some(ref vfs_file) = file.vfs_file {
                match vfs_file.metadata() {
                    Ok(m) => m.size as i64,
                    Err(_) => 0,
                }
            } else { 0 }
        },
        _ => 0,
    };
    
    let new_offset = match whence {
        crate::posix::SEEK_SET => offset,
        crate::posix::SEEK_CUR => file.offset as i64 + offset,
        crate::posix::SEEK_END => current_size + offset,
        _ => return E_INVAL,
    };
    
    if new_offset < 0 {
        return E_INVAL;
    }
    
    file.offset = new_offset as usize;
    new_offset as isize
}

fn sys_dup2(oldfd: i32, newfd: i32) -> isize {
    if oldfd < 0 || newfd < 0 || newfd >= crate::file::NOFILE as i32 {
        return E_BADF;
    }
    
    if oldfd == newfd {
        // Check if oldfd is valid
        if process::fdlookup(oldfd).is_none() {
            return E_BADF;
        }
        return newfd as isize;
    }
    
    let file_idx = match process::fdlookup(oldfd) { Some(idx) => idx, None => return E_BADF };
    
    // Close newfd if open
    if process::fdlookup(newfd).is_some() {
        sys_close(newfd);
    }
    
    // Increment refcount
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            f.ref_count += 1;
        } else {
            return E_BADF;
        }
    }
    
    // Install into newfd
    if process::fdinstall(newfd, file_idx).is_err() {
        // Should not happen if we checked range and closed it
        return E_MFILE;
    }
    
    newfd as isize
}

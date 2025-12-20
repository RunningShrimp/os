#!/bin/bash

# Convert crate::syscall(num, [args...]) to crate::syscallN(num, args...)

for file in user/src/glib/*.rs; do
    # Count parameters in array and convert
    sed -i '' '
    # syscall with 1 parameter
    s/crate::syscall(\([^,]*\), \[\([^,]*\)\])/crate::syscall1(\1, \2)/g
    
    # syscall with 2 parameters  
    s/crate::syscall(\([^,]*\), \[\([^,]*\), \([^,]*\)\])/crate::syscall2(\1, \2, \3)/g
    
    # syscall with 3 parameters
    s/crate::syscall(\([^,]*\), \[\([^,]*\), \([^,]*\), \([^,]*\)\])/crate::syscall3(\1, \2, \3, \4)/g
    
    # syscall with 4 parameters
    s/crate::syscall(\([^,]*\), \[\([^,]*\), \([^,]*\), \([^,]*\), \([^,]*\)\])/crate::syscall4(\1, \2, \3, \4, \5)/g
    
    # syscall with 5 parameters
    s/crate::syscall(\([^,]*\), \[\([^,]*\), \([^,]*\), \([^,]*\), \([^,]*\), \([^,]*\)\])/crate::syscall5(\1, \2, \3, \4, \5, \6)/g
    ' "$file"
done

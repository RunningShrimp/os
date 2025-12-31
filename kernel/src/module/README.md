# Kernel Module System

## Overview

The NOS kernel module system provides a complete dynamic kernel module loading framework, allowing the kernel to load and unload code at runtime. This system is similar to Linux kernel modules (LKMs) but designed with Rust's safety guarantees.

## Architecture

The module system consists of three main components:

### 1. Loader (`loader.rs`)

Handles ELF module parsing, relocation, and symbol resolution.

**Key Types:**
- `ElfModule`: Represents a loaded ELF module with text, data, and bss sections
- `Symbol`: Represents a symbol with name, value, size, and type
- `SymbolTable`: Global kernel symbol table
- `LoadError`: Errors that can occur during loading
- `RelocError`: Errors that can occur during relocation

**Key Methods:**
```rust
// Load an ELF module
let module = ElfModule::load(data, name.to_string())?;

// Apply relocations
module.relocate(base_address)?;

// Resolve symbols
module.resolve_symbols(&kernel_symbols)?;

// Get symbols
let symbol = module.get_symbol("function_name");
```

### 2. Manager (`manager.rs`)

Manages the lifecycle of kernel modules including loading, unloading, and dependency tracking.

**Key Types:**
- `ModuleManager`: Central manager for all modules
- `LoadedModule`: Represents a loaded module with metadata
- `ModuleState`: Current state of a module (Loading, Active, Unloading)
- `ModuleInfo`: Public information about a module

**Key Methods:**
```rust
let mut manager = ModuleManager::new();

// Load a module
manager.load_module("mymodule", &module_data)?;

// Unload a module
manager.unload_module("mymodule")?;

// Get module information
let info = manager.get_module_info("mymodule");

// List all modules
let modules = manager.list_modules();

// Export/import symbols
manager.export_symbol(String::from("function"), 0x1000);
let addr = manager.import_symbol("function");
```

### 3. Framework (`framework.rs`)

Provides the trait definitions and macros for creating modules.

**Key Trait:**
```rust
pub trait Module {
    fn init(&mut self) -> ModuleResult<()>;
    fn cleanup(&mut self) -> ModuleResult<()>;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn author(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &[ModuleParameter];
    fn set_parameter(&mut self, name: &str, value: &str) -> ModuleResult<()>;
}
```

**Key Macro:**
```rust
declare_module!(MyModuleType);
```

This macro automatically generates:
- `_module_init()`: Initialization function
- `_module_cleanup()`: Cleanup function
- `_module_name()`: Module name accessor
- `_module_version()`: Module version accessor

## Creating a Module

### Basic Example

```rust
use kernel::module::framework::{Module, ModuleResult, declare_module};
use alloc::boxed::Box;

struct MyModule {
    counter: usize,
}

impl Default for MyModule {
    fn default() -> Self {
        Self { counter: 0 }
    }
}

impl Module for MyModule {
    fn init(&mut self) -> ModuleResult<()> {
        // Initialize the module
        self.counter = 42;
        Ok(())
    }

    fn cleanup(&mut self) -> ModuleResult<()> {
        // Clean up resources
        Ok(())
    }

    fn name(&self) -> &str {
        "mymodule"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn author(&self) -> &str {
        "Your Name"
    }

    fn description(&self) -> &str {
        "My awesome kernel module"
    }
}

// Declare the module
declare_module!(MyModule);
```

### Module with Parameters

```rust
use kernel::module::framework::{Module, ModuleResult, ModuleParameter, ParameterType, module_param};

struct MyModule {
    enabled: bool,
    threshold: i32,
}

impl Module for MyModule {
    // ... other methods ...

    fn parameters(&self) -> &[ModuleParameter] {
        &[
            module_param!(bool, "enabled", "true", "Enable the module"),
            module_param!(int, "threshold", "100", "Threshold value"),
        ]
    }

    fn set_parameter(&mut self, name: &str, value: &str) -> ModuleResult<()> {
        match name {
            "enabled" => {
                self.enabled = value.parse().map_err(|_| ModuleError::InvalidModule)?;
            }
            "threshold" => {
                self.threshold = value.parse().map_err(|_| ModuleError::InvalidModule)?;
            }
            _ => return Err(ModuleError::InvalidModule),
        }
        Ok(())
    }
}
```

## Module Lifecycle

### Loading Process

1. **Validation**: Module name and size are validated
2. **ELF Parsing**: The ELF file is parsed and sections are loaded
3. **Memory Allocation**: Memory is allocated for text, data, and bss sections
4. **Symbol Resolution**: External symbols are resolved against kernel symbols
5. **Dependency Resolution**: Module dependencies are identified and loaded
6. **Relocation**: Relocations are applied to the module
7. **Initialization**: The module's `init()` function is called
8. **Activation**: Module state is set to Active

### Unloading Process

1. **Reference Check**: Verify no other modules depend on this module
2. **Cleanup**: Call the module's `cleanup()` function
3. **Dependency Update**: Decrement reference counts of dependencies
4. **Memory Release**: Free module memory
5. **Removal**: Remove from module list

## Symbol Resolution

### Kernel Symbols

The kernel exports a set of symbols that modules can use:

```rust
// Common kernel functions
printk, kmalloc, kfree, memcpy, memset, strlen, strcmp, strcpy

// Additional symbols can be exported at runtime:
manager.export_symbol(String::from("my_function"), addr);
```

### Module Symbols

Modules can export symbols that other modules can use:

```rust
// In module A
#[no_mangle]
pub extern "C" fn module_a_function() -> i32 {
    42
}

// In module B, reference module A's symbol
// The loader will resolve this automatically
extern "C" {
    fn module_a_function() -> i32;
}
```

## Security Considerations

### Module Signature Verification (TODO)

Currently, the module system does not verify module signatures. Future versions should:

1. Verify cryptographic signatures
2. Check module integrity
3. Validate module permissions
4. Enforce module sandboxing

### Memory Safety

The module system leverages Rust's type system to ensure memory safety:

- All module memory is managed through Rust's ownership system
- Symbol resolution is type-safe
- Module cleanup is guaranteed through Drop traits

### Capability Checks (TODO)

Future versions should implement:

- Capability-based access control
- Permission checks for sensitive operations
- Resource limits and quotas

## Limitations

### Current Limitations

1. **Relocation**: Only basic relocations are supported
2. **Architecture**: Only x86_64 is supported
3. **Module Signing**: No signature verification
4. **Sandboxing**: No module isolation
5. **Versioning**: Limited version compatibility checking

### Future Enhancements

1. Add support for more architectures (ARM64, RISC-V)
2. Implement module signing and verification
3. Add module sandboxing and isolation
4. Support for module parameter types beyond strings
5. Module version compatibility checking
6. Hot patching support
7. Module dependency graphs and visualization

## Testing

### Unit Tests

Run unit tests for the module system:

```bash
cargo test --package kernel --lib module
```

### Integration Tests

Run the full module test suite:

```bash
cargo test --package kernel --test module_tests
```

### Example Module

See `kernel/src/module/example.rs` for a complete example module with tests.

## Configuration

### Build Features

The module system is always enabled in the kernel. To use it:

```toml
[dependencies]
kernel = { path = "../kernel" }
```

### Runtime Configuration

Module parameters can be configured at runtime:

```rust
// Set a parameter
module.set_parameter("enabled", "true")?;

// Get current value
let param = module.parameters().iter()
    .find(|p| p.name == "enabled")
    .unwrap();
println!("enabled = {}", param.value);
```

## Error Handling

The module system uses Rust's Result type for error handling:

```rust
use kernel::module::{LoadError, RelocError, ModuleError};

match module_manager.load_module(name, data) {
    Ok(()) => println!("Module loaded successfully"),
    Err(Error::Load(LoadError::InvalidElf)) => eprintln!("Invalid ELF format"),
    Err(Error::Reloc(RelocError::SymbolNotFound(name))) => {
        eprintln!("Symbol not found: {}", name)
    }
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## Performance

### Memory Overhead

- Per module: ~1-2 KB for metadata
- Symbol table: O(n) where n is the number of symbols

### Loading Time

- ELF parsing: O(m) where m is the number of sections
- Symbol resolution: O(n*k) where n is symbols and k is loaded modules
- Relocation: O(r) where r is the number of relocations

### Optimization Tips

1. Minimize exported symbols
2. Use lazy loading for optional features
3. Cache symbol lookups when possible
4. Batch module operations

## Examples

See the following files for complete examples:

- `kernel/src/module/example.rs`: Example module implementation
- `kernel/tests/module_tests.rs`: Comprehensive test suite
- `kernel/src/module/loader.rs`: ELF loading examples
- `kernel/src/module/manager.rs`: Module management examples

## Related Documentation

- [ELF Specification](https://refspecs.linuxfoundation.org/elf/elf.pdf)
- [Linux Kernel Module Programming Guide](https://sysprog21.github.io/lkmpg/)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)

## License

This module system is part of the NOS kernel project.

#!/bin/bash

# NOS 开发工具脚本
# 提供常用的开发任务自动化

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否在正确的目录
check_directory() {
    if [ ! -f "Cargo.toml" ]; then
        print_error "请在NOS项目根目录下运行此脚本"
        exit 1
    fi
}

# 安装开发工具
install_dev_tools() {
    print_info "安装开发工具..."

    # 安装rustfmt
    if ! command -v rustfmt &> /dev/null; then
        print_info "安装rustfmt..."
        rustup component add rustfmt
    fi

    # 安装clippy
    if ! command -v cargo-clippy &> /dev/null; then
        print_info "安装clippy..."
        rustup component add clippy
    fi

    print_success "开发工具安装完成"
}

# 代码格式化
format_code() {
    print_info "格式化代码..."

    # 检查rustfmt是否存在
    if ! command -v rustfmt &> /dev/null; then
        print_error "rustfmt未安装，请运行: ./scripts/dev-tools.sh install"
        exit 1
    fi

    # 格式化所有Rust代码
    find . -name "*.rs" -type f -exec rustfmt {} \;

    # 检查格式化是否正确
    if rustfmt --check .; then
        print_success "代码格式正确"
    else
        print_warning "代码格式已修正"
        rustfmt .
    fi
}

# 运行clippy检查
run_clippy() {
    print_info "运行Clippy代码检查..."

    # 检查clippy是否存在
    if ! command -v cargo-clippy &> /dev/null; then
        print_error "clippy未安装，请运行: ./scripts/dev-tools.sh install"
        exit 1
    fi

    # 运行clippy，包含所有目标
    cargo clippy --all-targets --all-features -- -D warnings

    if [ $? -eq 0 ]; then
        print_success "Clippy检查通过"
    else
        print_warning "Clippy发现问题，请查看上面的输出"
    fi
}

# 运行测试
run_tests() {
    print_info "运行测试套件..."

    # 运行单元测试
    cargo test --lib

    # 如果启用了内核测试，也运行集成测试
    if [ "$1" = "--integration" ]; then
        print_info "运行集成测试..."
        cargo test --test integration_tests
    fi

    if [ $? -eq 0 ]; then
        print_success "所有测试通过"
    else
        print_error "测试失败"
        exit 1
    fi
}

# 检查代码覆盖率
check_coverage() {
    print_info "检查代码覆盖率..."

    # 安装grcov工具
    if ! cargo install grcov &> /dev/null; then
        print_info "grcov已安装"
    else
        print_info "安装grcov..."
        cargo install grcov
    fi

    # 设置环境变量
    export RUSTFLAGS="-Zinstrument-coverage"
    export LLVM_PROFILE_FILE="target/coverage/coverage-%p-%m.profraw"

    # 清理旧的覆盖率数据
    rm -rf target/coverage/
    mkdir -p target/coverage/

    # 运行测试并生成覆盖率数据
    cargo test

    # 生成覆盖率报告
    grcov target/coverage/ --llvm --branch --output-dir target/coverage/

    print_info "覆盖率报告生成在: target/coverage/"
}

# 构建内核
build_kernel() {
    print_info "构建NOS内核..."

    # 检查目标架构
    ARCH=${1:-$(uname -m)}

    case $ARCH in
        "x86_64")
            cargo build --target x86_64-unknown-none
            ;;
        "aarch64")
            cargo build --target aarch64-unknown-none
            ;;
        "riscv64")
            cargo build --target riscv64gc-unknown-none-elf
            ;;
        *)
            print_error "不支持的架构: $ARCH"
            exit 1
            ;;
    esac

    if [ $? -eq 0 ]; then
        print_success "内核构建成功"
    else
        print_error "内核构建失败"
        exit 1
    fi
}

# 运行代码质量检查
quality_check() {
    print_info "运行代码质量检查..."

    # 1. 格式化检查
    format_code

    # 2. Clippy检查
    run_clippy

    # 3. 运行测试
    run_tests

    print_success "代码质量检查完成"
}

# 生成文档
generate_docs() {
    print_info "生成API文档..."

    # 生成文档
    cargo doc --all --no-deps

    if [ $? -eq 0 ]; then
        print_success "文档生成完成，查看: target/doc/"
    else
        print_error "文档生成失败"
        exit 1
    fi
}

# 性能分析
performance_analysis() {
    print_info "运行性能分析..."

    # 安装性能分析工具
    if ! command -v perf &> /dev/null; then
        print_warning "perf未安装，跳过性能分析"
        return
    fi

    # 运行性能测试
    if [ -d "benchmarks" ]; then
        cargo bench
    fi

    # 分析内核性能
    perf record --call-graph=dwarf cargo test
    perf report

    print_info "性能分析完成"
}

# 内存泄漏检查
memory_check() {
    print_info "运行内存检查..."

    # 安装valgrind
    if ! command -v valgrind &> /dev/null; then
        print_warning "valgrind未安装，跳过内存检查"
        return
    fi

    # 运行valgrind
    valgrind --leak-check=full --show-leak-kinds=all cargo test

    print_info "内存检查完成"
}

# 清理构建产物
clean() {
    print_info "清理构建产物..."

    cargo clean
    rm -rf target/coverage/

    print_success "清理完成"
}

# 检查安全漏洞
security_check() {
    print_info "运行安全检查..."

    # 检查常见的安全问题
    cargo clippy --all-targets -- -D warnings \
        -W clippy::integer_arithmetic \
        -W clippy::overflow_check_conditional \
        -W clippy::indexing_slicing \
        -W clippy::out_of_bounds_indexing \
        -W clippy::panic \
        -W clippy::unimplemented \
        -W clippy::todo

    # 检查依赖项漏洞
    if command -v cargo-audit &> /dev/null; then
        cargo audit
    else
        print_warning "cargo-audit未安装，跳过依赖项漏洞检查"
    fi

    print_success "安全检查完成"
}

# 更新依赖
update_deps() {
    print_info "更新依赖项..."

    cargo update

    # 检查更新后的安全性
    security_check

    print_success "依赖更新完成"
}

# 显示帮助信息
show_help() {
    echo "NOS 开发工具脚本"
    echo ""
    echo "用法: $0 <命令> [选项]"
    echo ""
    echo "命令:"
    echo "  install         安装开发工具"
    echo "  format           格式化代码"
    echo "  check           运行Clippy检查"
    echo "  test            运行测试"
    echo "  coverage        检查代码覆盖率"
    echo "  build [arch]    构建内核 (默认: x86_64)"
    echo "  quality         运行完整的代码质量检查"
    echo "  docs            生成API文档"
    echo "  performance     性能分析"
    echo "  memory          内存泄漏检查"
    echo "  clean           清理构建产物"
    echo "  security        安全漏洞检查"
    echo "  update          更新依赖项"
    echo "  help            显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0 install"
    echo "  $0 quality"
    echo "  $0 build aarch64"
    echo "  $0 test --integration"
}

# 主函数
main() {
    check_directory

    case "${1:-help}" in
        "install")
            install_dev_tools
            ;;
        "format")
            format_code
            ;;
        "check"|"clippy")
            run_clippy
            ;;
        "test")
            run_tests "$2"
            ;;
        "coverage")
            check_coverage
            ;;
        "build")
            build_kernel "$2"
            ;;
        "quality")
            quality_check
            ;;
        "docs")
            generate_docs
            ;;
        "performance")
            performance_analysis
            ;;
        "memory")
            memory_check
            ;;
        "clean")
            clean
            ;;
        "security")
            security_check
            ;;
        "update")
            update_deps
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            print_error "未知命令: $1"
            show_help
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@"
# 存根跟踪系统说明

## 概述

本文档说明如何建立和维护存根跟踪系统，用于系统化跟踪项目中的TODO/FIXME/STUB标记。

## 工具和脚本

### 1. 存根扫描脚本

位置: `scripts/scan_stubs.sh`

功能:
- 扫描项目中所有TODO/FIXME/STUB标记
- 生成统计报告
- 按模块分类输出

使用方法:
```bash
./scripts/scan_stubs.sh
```

输出文件: `docs/STUB_REPORT.md`

### 2. 存根跟踪文档

位置: `docs/STUB_TRACKING.md`

内容:
- 已替换的存根列表
- 剩余存根清单
- 优先级分类
- 替换计划

## 建立跟踪系统

### 方案1: GitHub Issues

1. 为每个P0级别的存根创建Issue
2. 使用标签分类:
   - `priority/P0` - 高优先级
   - `priority/P1` - 中优先级
   - `priority/P2` - 低优先级
   - `module/syscalls` - 系统调用模块
   - `module/posix` - POSIX模块
   - `module/vfs` - 文件系统模块
   - `type/todo` - TODO标记
   - `type/fixme` - FIXME标记
   - `type/stub` - STUB标记

3. Issue模板:
```markdown
## 存根描述
[描述存根的功能和用途]

## 位置
文件: `kernel/src/xxx.rs`
行号: 123

## 当前实现
[描述当前的存根实现]

## 预期实现
[描述预期的完整实现]

## 优先级
- [ ] P0 - 高优先级
- [ ] P1 - 中优先级
- [ ] P2 - 低优先级

## 依赖关系
[列出依赖的其他存根或功能]

## 验收标准
- [ ] 功能完整实现
- [ ] 通过单元测试
- [ ] 通过集成测试
- [ ] 文档更新
```

### 方案2: 项目管理工具

使用项目管理工具（如Jira、Trello等）创建看板:

**看板列**:
- Backlog
- P0 - 高优先级
- P1 - 中优先级
- P2 - 低优先级
- In Progress
- Review
- Done

**卡片字段**:
- 模块
- 文件路径
- 行号
- 优先级
- 状态
- 负责人
- 预计完成时间

### 方案3: 本地跟踪文件

使用Markdown文件维护跟踪列表:

1. 创建 `docs/STUB_ISSUES.md`
2. 为每个存根创建条目
3. 使用Git提交跟踪变更

## 自动化流程

### CI/CD集成

在CI/CD流程中添加存根检查:

```yaml
# .github/workflows/stub-check.yml
name: Stub Check

on:
  pull_request:
    paths:
      - 'kernel/src/**/*.rs'
  schedule:
    - cron: '0 0 * * 0'  # 每周日运行

jobs:
  scan-stubs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Scan stubs
        run: ./scripts/scan_stubs.sh
      - name: Check stub count
        run: |
          COUNT=$(grep -c "总计" docs/STUB_REPORT.md | awk '{print $2}')
          if [ "$COUNT" -gt 350 ]; then
            echo "警告: 存根数量超过350个"
            exit 1
          fi
```

### 定期扫描

设置定期扫描任务:

```bash
# crontab
0 0 * * 0 cd /path/to/nos && ./scripts/scan_stubs.sh && git add docs/STUB_REPORT.md && git commit -m "Update stub report" || true
```

## 优先级分类标准

### P0 - 高优先级
- 影响核心功能
- 影响系统稳定性
- 阻塞其他功能实现
- 安全相关

### P1 - 中优先级
- 影响功能完整性
- 影响POSIX兼容性
- 影响性能

### P2 - 低优先级
- 代码组织
- 工具和辅助功能
- 文档完善

## 更新流程

1. **开发时**: 添加TODO/FIXME/STUB时，在STUB_TRACKING.md中记录
2. **完成时**: 更新STUB_TRACKING.md，标记为已完成
3. **定期**: 运行扫描脚本，更新报告
4. **审查**: 定期审查存根清单，调整优先级

## 最佳实践

1. **明确标记**: 使用明确的TODO/FIXME/STUB标记
2. **添加注释**: 说明存根的原因和预期实现
3. **设置优先级**: 在注释中标注优先级
4. **跟踪依赖**: 记录存根之间的依赖关系
5. **定期审查**: 定期审查和更新存根状态

## 示例

### 好的存根标记
```rust
// TODO(P0): 实现完整的权限检查逻辑
// 当前: 只检查基本权限
// 预期: 支持ACL和扩展属性
// 依赖: permission模块完成
fn check_permission(...) {
    // stub implementation
}
```

### 不好的存根标记
```rust
// TODO
fn check_permission(...) {
    // stub
}
```


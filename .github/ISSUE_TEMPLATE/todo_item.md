---
name: Todo 项目
description: 将实施计划中的条目拆分为可执行 Issue
title: "[Todo] 标题"
labels: ["todo"]
assignees: []
body:
  - type: input
    id: summary
    attributes:
      label: 概述
      description: 简要说明目标和业务背景
      placeholder: 例如：补齐 POSIX 信号处理，支持常用场景
    validations:
      required: true
  - type: textarea
    id: acceptance
    attributes:
      label: 验收标准
      description: 可量化的验收条件与覆盖要求
      placeholder: 例如：通过 N 项用例；与 Linux 语义一致；CI 绿色
    validations:
      required: true
  - type: textarea
    id: tasks
    attributes:
      label: 子任务
      description: 列出实现步骤（可勾选）
      value: "- [ ] 设计与接口评审\n- [ ] 实现与单元测试\n- [ ] 基准与性能验证\n- [ ] 文档与示例补充"
    validations:
      required: true
  - type: textarea
    id: testplan
    attributes:
      label: 测试计划
      description: 行为/兼容/性能测试的设计与覆盖
      placeholder: 例如：microbench、I/O/调度基准、行为用例
    validations:
      required: true
  - type: dropdown
    id: priority
    attributes:
      label: 优先级
      options:
        - P0
        - P1
        - P2
    validations:
      required: true
---


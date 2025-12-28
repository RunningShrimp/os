#!/bin/bash

# 清理所有 #![allow(...)] 属性的脚本

echo "开始清理 #![allow(...)] 属性..."

# 查找所有 .rs 文件并删除包含 allow 属性的行
find kernel/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(dead_code)\]/d' {} \;
find kernel/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_imports)\]/d' {} \;
find kernel/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_variables)\]/d' {} \;

# 也处理其他可能的 crate
find nos-api/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(dead_code)\]/d' {} \;
find nos-api/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_imports)\]/d' {} \;
find nos-api/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_variables)\]/d' {} \;

find nos-syscalls/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(dead_code)\]/d' {} \;
find nos-syscalls/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_imports)\]/d' {} \;
find nos-syscalls/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_variables)\]/d' {} \;

find nos-error-handling/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(dead_code)\]/d' {} \;
find nos-error-handling/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_imports)\]/d' {} \;
find nos-error-handling/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_variables)\]/d' {} \;

find nos-services/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(dead_code)\]/d' {} \;
find nos-services/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_imports)\]/d' {} \;
find nos-services/src -name "*.rs" -type f -exec sed -i '' '/#!\[allow(unused_variables)\]/d' {} \;

echo "清理完成！"
echo "已删除所有 #![allow(dead_code)] 属性"
echo "已删除所有 #![allow(unused_imports)] 属性"
echo "已删除所有 #![allow(unused_variables)] 属性"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>

// 测试基本I/O
void test_stdio() {
    printf("=== C标准库测试 ===\n");

    // 测试puts
    puts("Hello from NOS C Standard Library!");

    // 测试putchar
    putchar('*');
    putchar(' ');
    putchar('\n');

    // 测试getchar
    printf("请输入一个字符: ");
    int c = getchar();
    printf("你输入的字符是: %c\n", c);
}

// 测试内存管理
void test_memory() {
    printf("\n=== 内存管理测试 ===\n");

    // 测试malloc
    char *buffer = malloc(256);
    if (buffer == NULL) {
        printf("内存分配失败\n");
        return;
    }

    printf("成功分配256字节内存\n");

    // 测试memset
    memset(buffer, 0, 256);
    strcpy(buffer, "NOS C Standard Library");
    printf("字符串内容: %s\n", buffer);

    // 测试realloc
    char *new_buffer = realloc(buffer, 512);
    if (new_buffer != NULL) {
        printf("成功重新分配到512字节\n");
        strcpy(new_buffer, "NOS C Standard Library - Extended");
        printf("扩展后内容: %s\n", new_buffer);
        free(new_buffer);
    } else {
        printf("内存重新分配失败\n");
        free(buffer);
    }

    // 测试calloc
    int *int_array = calloc(10, sizeof(int));
    if (int_array != NULL) {
        printf("成功分配10个整数的数组\n");
        for (int i = 0; i < 10; i++) {
            int_array[i] = i * i;
        }
        printf("数组元素平方: ");
        for (int i = 0; i < 10; i++) {
            printf("%d ", int_array[i]);
        }
        printf("\n");
        free(int_array);
    }
}

// 测试字符串处理
void test_strings() {
    printf("\n=== 字符串处理测试 ===\n");

    const char *str1 = "Hello";
    const char *str2 = "NOS";
    char *buffer3 = malloc(100);

    if (buffer3 == NULL) {
        printf("内存分配失败\n");
        return;
    }

    // 测试strlen
    printf("strlen(\"Hello\") = %zu\n", strlen(str1));
    printf("strlen(\"NOS\") = %zu\n", strlen(str2));

    // 测试strcpy
    strcpy(buffer3, str1);
    printf("strcpy(buffer3, \"%s\") = %s\n", str1, buffer3);

    // 测试strcat
    strcat(buffer3, " ");
    strcat(buffer3, str2);
    printf("strcat结果: %s\n", buffer3);

    // 测试strcmp
    printf("strcmp(\"Hello\", \"NOS\") = %d\n", strcmp(str1, str2));
    printf("strcmp(\"Hello\", \"Hello\") = %d\n", strcmp(str1, str1));

    // 测试strncmp
    printf("strncmp(\"Hello\", \"Help\", 3) = %d\n", strncmp(str1, "Help", 3));

    free(buffer3);
}

// 测试数学函数
void test_math() {
    printf("\n=== 数学函数测试 ===\n");

    double pi = 3.141592653589793;

    printf("π = %.6f\n", pi);
    printf("sin(π/2) = %.6f\n", sin(pi / 2.0));
    printf("cos(π/2) = %.6f\n", cos(pi / 2.0));
    printf("tan(π/4) = %.6f\n", tan(pi / 4.0));

    printf("exp(1) = %.6f\n", exp(1.0));
    printf("log(e) = %.6f\n", log(2.718281828));
    printf("pow(2, 8) = %.6f\n", pow(2.0, 8.0));
    printf("sqrt(16) = %.6f\n", sqrt(16.0));

    printf("ceil(3.14) = %.0f\n", ceil(3.14));
    printf("floor(3.14) = %.0f\n", floor(3.14));
    printf("round(3.14) = %.0f\n", round(3.14));
    printf("fabs(-5.5) = %.1f\n", fabs(-5.5));
}

// 测试文件I/O
void test_file_io() {
    printf("\n=== 文件I/O测试 ===\n");

    const char *filename = "/tmp/test_file.txt";

    // 测试文件创建和写入
    FILE *file = fopen(filename, "w");
    if (file == NULL) {
        printf("无法创建文件 %s: %s\n", filename, strerror(errno));
        return;
    }

    printf("成功创建文件: %s\n", filename);

    fprintf(file, "NOS C Standard Library File I/O Test\n");
    fprintf(file, "时间戳: %ld\n", time(NULL));
    fprintf(file, "PID: %d\n", getpid());

    fclose(file);

    // 测试文件读取
    file = fopen(filename, "r");
    if (file == NULL) {
        printf("无法打开文件 %s: %s\n", filename, strerror(errno));
        return;
    }

    printf("成功打开文件: %s\n", filename);

    char line[256];
    while (fgets(line, sizeof(line), file) != NULL) {
        printf("文件内容: %s", line);
    }

    fclose(file);

    // 测试文件删除
    if (unlink(filename) == 0) {
        printf("成功删除文件: %s\n", filename);
    } else {
        printf("无法删除文件 %s: %s\n", filename, strerror(errno));
    }
}

// 测试系统调用
void test_syscalls() {
    printf("\n=== 系统调用测试 ===\n");

    printf("进程ID: %d\n", getpid());
    printf("父进程ID: %d\n", getppid());

    // 测试环境变量
    const char *path = getenv("PATH");
    if (path) {
        printf("PATH = %s\n", path);
    } else {
        printf("PATH 环境变量未设置\n");
    }

    const char *user = getenv("USER");
    if (user) {
        printf("USER = %s\n", user);
    } else {
        printf("USER 环境变量未设置\n");
    }

    // 测试随机数
    srand(time(NULL));
    printf("随机数(0-99): %d\n", rand() % 100);
    printf("随机数(0-999): %d\n", rand() % 1000);

    // 测试时间
    time_t current_time = time(NULL);
    printf("当前时间: %s", ctime(&current_time));

    // 测试sleep
    printf("睡眠1秒...\n");
    sleep(1);
    printf("睡眠结束\n");
}

// 计算斐波那契数列
void fibonacci(int n) {
    if (n <= 0) {
        return;
    }

    int a = 0, b = 1, c;

    printf("斐波那契数列前%d项: ", n);

    for (int i = 0; i < n && i < 20; i++) {
        printf("%d ", a);
        c = a + b;
        a = b;
        b = c;
    }

    printf("\n");
}

// 排序算法示例（冒泡排序）
void bubble_sort(int *array, int size) {
    for (int i = 0; i < size - 1; i++) {
        for (int j = 0; j < size - i - 1; j++) {
            if (array[j] > array[j + 1]) {
                // 交换元素
                int temp = array[j];
                array[j] = array[j + 1];
                array[j + 1] = temp;
            }
        }
    }
}

// 测试排序
void test_sorting() {
    printf("\n=== 排序算法测试 ===\n");

    int data[] = {64, 34, 25, 12, 22, 11, 90, 88, 76, 50};
    int size = sizeof(data) / sizeof(data[0]);

    printf("原始数组: ");
    for (int i = 0; i < size; i++) {
        printf("%d ", data[i]);
    }
    printf("\n");

    bubble_sort(data, size);

    printf("排序后数组: ");
    for (int i = 0; i < size; i++) {
        printf("%d ", data[i]);
    }
    printf("\n");
}

// 错误处理示例
void test_error_handling() {
    printf("\n=== 错误处理测试 ===\n");

    // 测试无效文件描述符
    printf("读取无效文件描述符: ");
    int bytes_read = read(-1, NULL, 0);
    if (bytes_read == -1) {
        printf("错误: %s\n", strerror(errno));
    }

    // 测试无效内存访问
    printf("访问NULL指针: ");
    char *null_ptr = NULL;
    size_t len = strlen(null_ptr);
    printf("长度: %zu (应该是0)\n", len);

    // 测试无效参数
    printf("开无效文件: ");
    int fd = open("/nonexistent/file", O_RDONLY);
    if (fd == -1) {
        printf("错误: %s\n", strerror(errno));
    }
}

int main() {
    printf("NOS C Standard Library 测试程序\n");
    printf("编译时间: %s %s\n", __DATE__, __TIME__);

    test_stdio();
    test_memory();
    test_strings();
    test_math();
    test_file_io();
    test_syscalls();

    printf("\n=== 算法测试 ===\n");
    fibonacci(15);
    test_sorting();

    test_error_handling();

    printf("\n=== 程序即将结束 ===\n");
    printf("感谢使用NOS C标准库!\n");

    return EXIT_SUCCESS;
}
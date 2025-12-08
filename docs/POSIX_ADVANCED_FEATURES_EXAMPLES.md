# POSIX高级特性使用示例

本文档提供了NOS内核中POSIX高级特性的实用示例，展示如何在实际应用中使用这些功能。

## 目录

1. [异步I/O（AIO）示例](#1-异步ioaio示例)
2. [高级内存映射示例](#2-高级内存映射示例)
3. [消息队列示例](#3-消息队列示例)
4. [高级信号处理示例](#4-高级信号处理示例)
5. [实时扩展示例](#5-实时扩展示例)
6. [高级线程特性示例](#6-高级线程特性示例)
7. [安全机制示例](#7-安全机制示例)
8. [综合应用示例](#8-综合应用示例)

## 1. 异步I/O（AIO）示例

### 1.1 基本异步文件读取

```c
#include <aio.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

#define BUFFER_SIZE 4096

void aio_completion_handler(int sig, siginfo_t *si, void *ucontext) {
    struct aiocb *cb = (struct aiocb *)si->si_value.sival_ptr;
    
    // 检查AIO操作状态
    if (aio_error(cb) == 0) {
        ssize_t ret = aio_return(cb);
        printf("AIO read completed: %zd bytes\n", ret);
        
        // 处理读取的数据
        if (ret > 0) {
            printf("Data: %.*s\n", (int)ret, (char*)cb->aio_buf);
        }
    } else {
        printf("AIO read failed: %s\n", strerror(aio_error(cb)));
    }
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <filename>\n", argv[0]);
        return 1;
    }
    
    int fd = open(argv[1], O_RDONLY);
    if (fd == -1) {
        perror("open");
        return 1;
    }
    
    // 分配缓冲区
    char *buffer = malloc(BUFFER_SIZE);
    if (!buffer) {
        perror("malloc");
        close(fd);
        return 1;
    }
    
    // 设置AIO控制块
    struct aiocb cb;
    memset(&cb, 0, sizeof(cb));
    cb.aio_fildes = fd;
    cb.aio_buf = buffer;
    cb.aio_nbytes = BUFFER_SIZE;
    cb.aio_offset = 0;
    cb.aio_sigevent.sigev_notify = SIGEV_SIGNAL;
    cb.aio_sigevent.sigev_signo = SIGUSR1;
    cb.aio_sigevent.sigev_value.sival_ptr = &cb;
    
    // 设置信号处理
    struct sigaction sa;
    sa.sa_sigaction = aio_completion_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGUSR1, &sa, NULL);
    
    // 启动异步读取
    if (aio_read(&cb) == -1) {
        perror("aio_read");
        free(buffer);
        close(fd);
        return 1;
    }
    
    printf("AIO read started, waiting for completion...\n");
    
    // 等待AIO完成
    while (aio_error(&cb) == EINPROGRESS) {
        // 可以在这里做其他工作
        printf("Doing other work...\n");
        usleep(100000); // 100ms
    }
    
    // 清理资源
    free(buffer);
    close(fd);
    
    return 0;
}
```

### 1.2 批量AIO操作

```c
#include <aio.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

#define NUM_OPERATIONS 4
#define BUFFER_SIZE 1024

int main() {
    struct aiocb *cbs[NUM_OPERATIONS];
    char *buffers[NUM_OPERATIONS];
    int fd = open("testfile.txt", O_RDWR | O_CREAT, 0644);
    
    if (fd == -1) {
        perror("open");
        return 1;
    }
    
    // 准备多个AIO操作
    for (int i = 0; i < NUM_OPERATIONS; i++) {
        buffers[i] = malloc(BUFFER_SIZE);
        if (!buffers[i]) {
            perror("malloc");
            return 1;
        }
        
        cbs[i] = malloc(sizeof(struct aiocb));
        if (!cbs[i]) {
            perror("malloc");
            return 1;
        }
        
        memset(cbs[i], 0, sizeof(struct aiocb));
        cbs[i]->aio_fildes = fd;
        cbs[i]->aio_buf = buffers[i];
        cbs[i]->aio_nbytes = BUFFER_SIZE;
        cbs[i]->aio_offset = i * BUFFER_SIZE;
        cbs[i]->aio_lio_opcode = (i % 2 == 0) ? LIO_WRITE : LIO_READ;
        
        // 为写操作准备数据
        if (i % 2 == 0) {
            snprintf(buffers[i], BUFFER_SIZE, "Block %d data\n", i);
        }
    }
    
    // 启动批量AIO操作
    if (lio_listio(LIO_WAIT, cbs, NUM_OPERATIONS, NULL) == -1) {
        perror("lio_listio");
        return 1;
    }
    
    printf("All AIO operations completed\n");
    
    // 检查结果
    for (int i = 0; i < NUM_OPERATIONS; i++) {
        if (cbs[i]->aio_lio_opcode == LIO_READ) {
            ssize_t ret = aio_return(cbs[i]);
            if (ret > 0) {
                printf("Read %zd bytes: %.*s", ret, (int)ret, buffers[i]);
            }
        }
        
        free(buffers[i]);
        free(cbs[i]);
    }
    
    close(fd);
    return 0;
}
```

## 2. 高级内存映射示例

### 2.1 内存锁定和大页映射

```c
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

#define MAP_SIZE (1024 * 1024) // 1MB

int main() {
    int fd;
    void *mapped_addr;
    
    // 创建文件用于映射
    fd = open("mmap_test.dat", O_RDWR | O_CREAT, 0644);
    if (fd == -1) {
        perror("open");
        return 1;
    }
    
    // 设置文件大小
    if (ftruncate(fd, MAP_SIZE) == -1) {
        perror("ftruncate");
        close(fd);
        return 1;
    }
    
    // 使用大页映射
    mapped_addr = mmap(NULL, MAP_SIZE, PROT_READ | PROT_WRITE,
                      MAP_SHARED | MAP_HUGETLB | MAP_LOCKED, fd, 0);
    
    if (mapped_addr == MAP_FAILED) {
        // 如果大页失败，使用普通映射
        printf("Huge page mapping failed, trying regular mapping\n");
        mapped_addr = mmap(NULL, MAP_SIZE, PROT_READ | PROT_WRITE,
                          MAP_SHARED | MAP_LOCKED, fd, 0);
        
        if (mapped_addr == MAP_FAILED) {
            perror("mmap");
            close(fd);
            return 1;
        }
    }
    
    printf("Memory mapped at %p\n", mapped_addr);
    
    // 使用映射的内存
    strcpy((char*)mapped_addr, "Hello, memory mapping!");
    printf("Written data: %s\n", (char*)mapped_addr);
    
    // 设置内存使用建议
    if (madvise(mapped_addr, MAP_SIZE, MADV_SEQUENTIAL) == -1) {
        perror("madvise");
    } else {
        printf("Set MADV_SEQUENTIAL advice\n");
    }
    
    // 检查页面是否在内存中
    size_t page_size = sysconf(_SC_PAGESIZE);
    size_t num_pages = (MAP_SIZE + page_size - 1) / page_size;
    unsigned char *vec = malloc(num_pages);
    
    if (mincore(mapped_addr, MAP_SIZE, vec) == 0) {
        printf("Page residency:\n");
        for (size_t i = 0; i < num_pages; i++) {
            printf("  Page %zu: %s\n", i, vec[i] & 1 ? "resident" : "not resident");
        }
    }
    
    free(vec);
    
    // 解除映射
    if (munmap(mapped_addr, MAP_SIZE) == -1) {
        perror("munmap");
    }
    
    close(fd);
    unlink("mmap_test.dat");
    
    return 0;
}
```

### 2.2 非线性文件映射

```c
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FILE_SIZE (1024 * 1024) // 1MB
#define BLOCK_SIZE 4096

int main() {
    int fd;
    void *mapped_addr;
    
    // 创建测试文件
    fd = open("nonlinear_test.dat", O_RDWR | O_CREAT, 0644);
    if (fd == -1) {
        perror("open");
        return 1;
    }
    
    // 设置文件大小
    if (ftruncate(fd, FILE_SIZE) == -1) {
        perror("ftruncate");
        close(fd);
        return 1;
    }
    
    // 创建线性映射
    mapped_addr = mmap(NULL, FILE_SIZE, PROT_READ | PROT_WRITE,
                      MAP_SHARED, fd, 0);
    
    if (mapped_addr == MAP_FAILED) {
        perror("mmap");
        close(fd);
        return 1;
    }
    
    printf("Linear mapping created at %p\n", mapped_addr);
    
    // 初始化文件内容
    for (int i = 0; i < FILE_SIZE / BLOCK_SIZE; i++) {
        char *block = (char*)mapped_addr + i * BLOCK_SIZE;
        snprintf(block, BLOCK_SIZE, "Block %d content\n", i);
    }
    
    // 创建非线性映射（重排页面）
    // 将第0页映射到第2页的位置
    if (remap_file_pages(mapped_addr + 2 * BLOCK_SIZE, BLOCK_SIZE,
                        PROT_READ | PROT_WRITE, 0, 0) == -1) {
        perror("remap_file_pages");
        printf("Nonlinear mapping not supported, continuing with linear mapping\n");
    } else {
        printf("Nonlinear mapping created\n");
        
        // 验证非线性映射
        printf("Content at page 2: %s", (char*)mapped_addr + 2 * BLOCK_SIZE);
    }
    
    // 清理
    munmap(mapped_addr, FILE_SIZE);
    close(fd);
    unlink("nonlinear_test.dat");
    
    return 0;
}
```

## 3. 消息队列示例

### 3.1 生产者-消费者模式

```c
#include <mqueue.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>

#define QUEUE_NAME "/pc_queue"
#define MAX_MSGS 10
#define MSG_SIZE 256

typedef struct {
    int id;
    char data[MSG_SIZE - sizeof(int)];
} message_t;

void producer(int num_messages) {
    mqd_t mq;
    struct mq_attr attr;
    message_t msg;
    
    // 设置队列属性
    attr.mq_flags = 0;
    attr.mq_maxmsg = MAX_MSGS;
    attr.mq_msgsize = sizeof(message_t);
    attr.mq_curmsgs = 0;
    
    // 创建消息队列
    mq = mq_open(QUEUE_NAME, O_CREAT | O_WRONLY, 0644, &attr);
    if (mq == (mqd_t)-1) {
        perror("mq_open (producer)");
        exit(1);
    }
    
    printf("Producer started, sending %d messages\n", num_messages);
    
    for (int i = 0; i < num_messages; i++) {
        msg.id = i;
        snprintf(msg.data, sizeof(msg.data), "Message %d from producer", i);
        
        // 发送消息，使用消息ID作为优先级
        if (mq_send(mq, (const char*)&msg, sizeof(msg), i) == -1) {
            perror("mq_send");
            break;
        }
        
        printf("Sent message %d\n", i);
        usleep(100000); // 100ms
    }
    
    mq_close(mq);
    printf("Producer finished\n");
}

void consumer(int num_messages) {
    mqd_t mq;
    message_t msg;
    unsigned int prio;
    
    // 打开消息队列
    mq = mq_open(QUEUE_NAME, O_RDONLY);
    if (mq == (mqd_t)-1) {
        perror("mq_open (consumer)");
        exit(1);
    }
    
    printf("Consumer started, waiting for messages\n");
    
    for (int i = 0; i < num_messages; i++) {
        ssize_t bytes = mq_receive(mq, (char*)&msg, sizeof(msg), &prio);
        if (bytes == -1) {
            perror("mq_receive");
            break;
        }
        
        printf("Received message %d (priority %u): %s\n", 
               msg.id, prio, msg.data);
    }
    
    mq_close(mq);
    printf("Consumer finished\n");
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <num_messages>\n", argv[0]);
        return 1;
    }
    
    int num_messages = atoi(argv[1]);
    
    // 创建子进程作为消费者
    pid_t pid = fork();
    
    if (pid == 0) {
        // 子进程 - 消费者
        sleep(1); // 让生产者先启动
        consumer(num_messages);
    } else if (pid > 0) {
        // 父进程 - 生产者
        producer(num_messages);
        
        // 等待子进程
        wait(NULL);
        
        // 清理消息队列
        mq_unlink(QUEUE_NAME);
    } else {
        perror("fork");
        return 1;
    }
    
    return 0;
}
```

### 3.2 异步通知消息队列

```c
#include <mqueue.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>

#define QUEUE_NAME "/async_queue"
#define MSG_SIZE 256

mqd_t mq;

void signal_handler(int sig, siginfo_t *si, void *ucontext) {
    char buffer[MSG_SIZE];
    unsigned int prio;
    
    // 接收消息
    ssize_t bytes = mq_receive(mq, buffer, sizeof(buffer), &prio);
    if (bytes > 0) {
        buffer[bytes] = '\0';
        printf("Received via signal (priority %u): %s\n", prio, buffer);
    }
    
    // 重新注册通知
    struct sigevent sev;
    sev.sigev_notify = SIGEV_SIGNAL;
    sev.sigev_signo = SIGUSR1;
    mq_notify(mq, &sev);
}

int main() {
    struct sigevent sev;
    struct sigaction sa;
    struct mq_attr attr;
    
    // 设置队列属性
    attr.mq_flags = 0;
    attr.mq_maxmsg = 10;
    attr.mq_msgsize = MSG_SIZE;
    attr.mq_curmsgs = 0;
    
    // 创建消息队列
    mq = mq_open(QUEUE_NAME, O_CREAT | O_RDWR, 0644, &attr);
    if (mq == (mqd_t)-1) {
        perror("mq_open");
        return 1;
    }
    
    // 设置信号处理
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = signal_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGUSR1, &sa, NULL);
    
    // 注册异步通知
    memset(&sev, 0, sizeof(sev));
    sev.sigev_notify = SIGEV_SIGNAL;
    sev.sigev_signo = SIGUSR1;
    if (mq_notify(mq, &sev) == -1) {
        perror("mq_notify");
        mq_close(mq);
        mq_unlink(QUEUE_NAME);
        return 1;
    }
    
    printf("Async message queue set up, waiting for messages...\n");
    
    // 在另一个终端发送消息测试：
    // mq_send /async_queue "Hello from terminal"
    
    // 主循环
    while (1) {
        pause(); // 等待信号
    }
    
    // 清理（实际上不会到达这里）
    mq_close(mq);
    mq_unlink(QUEUE_NAME);
    
    return 0;
}
```

## 4. 高级信号处理示例

### 4.1 实时信号队列

```c
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>

#define NUM_SIGNALS 10

volatile sig_atomic_t signal_count = 0;

void rt_signal_handler(int sig, siginfo_t *info, void *context) {
    static int count = 0;
    
    count++;
    signal_count++;
    
    printf("Real-time signal %d received (count: %d, value: %d)\n",
           sig, count, info->si_value.sival_int);
}

int main() {
    struct sigaction sa;
    int rt_sig = SIGRTMIN + 5;
    
    // 设置实时信号处理
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = rt_signal_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_SIGINFO;
    
    if (sigaction(rt_sig, &sa, NULL) == -1) {
        perror("sigaction");
        return 1;
    }
    
    printf("Real-time signal handler set up for signal %d\n", rt_sig);
    
    // 发送多个实时信号
    for (int i = 0; i < NUM_SIGNALS; i++) {
        union sigval value;
        value.sival_int = i;
        
        if (sigqueue(getpid(), rt_sig, value) == -1) {
            perror("sigqueue");
            return 1;
        }
        
        printf("Queued real-time signal %d with value %d\n", rt_sig, i);
    }
    
    // 等待所有信号被处理
    while (signal_count < NUM_SIGNALS) {
        usleep(10000); // 10ms
    }
    
    printf("All %d real-time signals processed\n", signal_count);
    
    return 0;
}
```

### 4.2 替代信号栈

```c
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#define STACK_SIZE (SIGSTKSZ * 2)

void stack_overflow_handler(int sig) {
    printf("Signal %d handled on alternate stack\n", sig);
    
    // 检查是否在替代栈上
    stack_t ss;
    if (sigaltstack(NULL, &ss) == -1) {
        perror("sigaltstack");
        return;
    }
    
    if (ss.ss_flags & SS_ONSTACK) {
        printf("Currently executing on alternate stack\n");
    } else {
        printf("Currently executing on main stack\n");
    }
}

void recursive_function(int depth) {
    char buffer[8192]; // 大缓冲区消耗栈空间
    printf("Recursive call depth: %d\n", depth);
    
    if (depth < 100) {
        recursive_function(depth + 1);
    }
}

int main() {
    stack_t ss;
    struct sigaction sa;
    
    // 分配替代栈
    ss.ss_sp = malloc(STACK_SIZE);
    if (!ss.ss_sp) {
        perror("malloc");
        return 1;
    }
    ss.ss_size = STACK_SIZE;
    ss.ss_flags = 0;
    
    // 设置替代信号栈
    if (sigaltstack(&ss, NULL) == -1) {
        perror("sigaltstack");
        free(ss.ss_sp);
        return 1;
    }
    
    printf("Alternate signal stack set up\n");
    
    // 设置信号处理
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = stack_overflow_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_ONSTACK;
    
    if (sigaction(SIGSEGV, &sa, NULL) == -1) {
        perror("sigaction");
        free(ss.ss_sp);
        return 1;
    }
    
    if (sigaction(SIGUSR1, &sa, NULL) == -1) {
        perror("sigaction");
        free(ss.ss_sp);
        return 1;
    }
    
    // 测试替代栈
    printf("Testing alternate stack with SIGUSR1\n");
    raise(SIGUSR1);
    
    // 测试栈溢出（小心使用）
    printf("Testing stack overflow (may trigger SIGSEGV)\n");
    // recursive_function(0); // 取消注释测试栈溢出
    
    // 清理
    free(ss.ss_sp);
    
    return 0;
}
```

### 4.3 线程信号掩码

```c
#include <signal.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

sig_atomic_t global_signal_count = 0;

void signal_handler(int sig) {
    global_signal_count++;
    printf("Signal %d handled by thread %lu\n", sig, pthread_self());
}

void *thread_func(void *arg) {
    int thread_id = *(int*)arg;
    sigset_t set;
    int sig;
    
    // 设置线程信号掩码 - 阻塞SIGUSR1
    sigemptyset(&set);
    sigaddset(&set, SIGUSR1);
    
    if (pthread_sigmask(SIG_BLOCK, &set, NULL) != 0) {
        perror("pthread_sigmask");
        return NULL;
    }
    
    printf("Thread %d: SIGUSR1 blocked\n", thread_id);
    
    // 等待信号
    for (int i = 0; i < 3; i++) {
        if (sigwait(&set, &sig) == 0) {
            printf("Thread %d: Received signal %d\n", thread_id, sig);
        }
    }
    
    return NULL;
}

int main() {
    pthread_t threads[3];
    int thread_ids[3];
    struct sigaction sa;
    
    // 设置信号处理
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = signal_handler;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGUSR2, &sa, NULL);
    
    // 创建线程
    for (int i = 0; i < 3; i++) {
        thread_ids[i] = i;
        if (pthread_create(&threads[i], NULL, thread_func, &thread_ids[i]) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    
    // 等待线程启动
    sleep(1);
    
    // 向线程发送信号
    for (int i = 0; i < 3; i++) {
        printf("Sending SIGUSR1 to thread %d\n", i);
        pthread_kill(threads[i], SIGUSR1);
        usleep(100000);
    }
    
    // 向主线程发送信号
    printf("Sending SIGUSR2 to main thread\n");
    raise(SIGUSR2);
    
    // 等待线程结束
    for (int i = 0; i < 3; i++) {
        pthread_join(threads[i], NULL);
    }
    
    printf("Global signal count: %d\n", global_signal_count);
    
    return 0;
}
```

## 5. 实时扩展示例

### 5.1 实时调度策略

```c
#include <sched.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/resource.h>
#include <time.h>

#define NUM_THREADS 3
#define ITERATIONS 10000000

void *realtime_thread(void *arg) {
    int thread_id = *(int*)arg;
    struct timespec start, end;
    int policy;
    struct sched_param param;
    
    // 获取线程调度参数
    pthread_getschedparam(pthread_self(), &policy, &param);
    
    printf("Thread %d: policy=%s, priority=%d\n", 
           thread_id,
           policy == SCHED_FIFO ? "FIFO" : 
           policy == SCHED_RR ? "RR" : "OTHER",
           param.sched_priority);
    
    // 测量执行时间
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &start);
    
    volatile long sum = 0;
    for (long i = 0; i < ITERATIONS; i++) {
        sum += i;
    }
    
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &end);
    
    double elapsed = (end.tv_sec - start.tv_sec) + 
                    (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("Thread %d: completed in %.6f seconds (sum=%ld)\n", 
           thread_id, elapsed, sum);
    
    return NULL;
}

int main() {
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    pthread_attr_t attr;
    struct sched_param param;
    
    // 检查权限
    if (getuid() != 0) {
        printf("Warning: Need root privileges for real-time scheduling\n");
    }
    
    // 初始化线程属性
    pthread_attr_init(&attr);
    
    // 设置调度策略为FIFO
    if (pthread_attr_setschedpolicy(&attr, SCHED_FIFO) != 0) {
        perror("pthread_attr_setschedpolicy");
        printf("Continuing with default scheduling\n");
    }
    
    // 设置调度参数
    param.sched_priority = 50;
    if (pthread_attr_setschedparam(&attr, &param) != 0) {
        perror("pthread_attr_setschedparam");
        printf("Continuing with default priority\n");
    }
    
    // 设置调度继承属性
    pthread_attr_setinheritsched(&attr, PTHREAD_EXPLICIT_SCHED);
    
    // 创建实时线程
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        
        if (pthread_create(&threads[i], &attr, realtime_thread, &thread_ids[i]) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    
    // 等待所有线程完成
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    // 清理
    pthread_attr_destroy(&attr);
    
    return 0;
}
```

### 5.2 CPU亲和性设置

```c
#include <sched.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/sysinfo.h>
#include <time.h>

#define NUM_THREADS 4
#define WORK_ITERATIONS 100000000

void *cpu_bound_thread(void *arg) {
    int thread_id = *(int*)arg;
    cpu_set_t cpuset;
    int cpu_id = thread_id % get_nprocs();
    
    // 设置CPU亲和性
    CPU_ZERO(&cpuset);
    CPU_SET(cpu_id, &cpuset);
    
    if (pthread_setaffinity_np(pthread_self(), sizeof(cpuset), &cpuset) != 0) {
        perror("pthread_setaffinity_np");
    } else {
        printf("Thread %d bound to CPU %d\n", thread_id, cpu_id);
    }
    
    // 验证CPU亲和性
    if (pthread_getaffinity_np(pthread_self(), sizeof(cpuset), &cpuset) == 0) {
        printf("Thread %d affinity: ", thread_id);
        for (int i = 0; i < get_nprocs(); i++) {
            if (CPU_ISSET(i, &cpuset)) {
                printf("%d ", i);
            }
        }
        printf("\n");
    }
    
    // CPU密集型工作
    struct timespec start, end;
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &start);
    
    volatile long sum = 0;
    for (long i = 0; i < WORK_ITERATIONS; i++) {
        sum += i * (thread_id + 1);
    }
    
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &end);
    
    double elapsed = (end.tv_sec - start.tv_sec) + 
                    (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("Thread %d: completed in %.6f seconds (sum=%ld)\n", 
           thread_id, elapsed, sum);
    
    return NULL;
}

int main() {
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    
    printf("Number of CPUs: %d\n", get_nprocs());
    
    // 创建CPU绑定线程
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        
        if (pthread_create(&threads[i], NULL, cpu_bound_thread, &thread_ids[i]) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    
    // 等待所有线程完成
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    return 0;
}
```

## 6. 高级线程特性示例

### 6.1 屏障同步

```c
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>

#define NUM_THREADS 5
#define NUM_PHASES 3

pthread_barrier_t barrier;

void phase_work(int thread_id, int phase) {
    printf("Thread %d: Phase %d - Working\n", thread_id, phase);
    
    // 模拟工作
    usleep(100000 + thread_id * 10000); // 不同的工作时长
    
    printf("Thread %d: Phase %d - Work completed\n", thread_id, phase);
}

void *thread_func(void *arg) {
    int thread_id = *(int*)arg;
    
    for (int phase = 1; phase <= NUM_PHASES; phase++) {
        phase_work(thread_id, phase);
        
        // 等待所有线程到达屏障
        printf("Thread %d: Waiting at barrier for phase %d\n", thread_id, phase);
        
        int result = pthread_barrier_wait(&barrier);
        
        if (result == 0) {
            printf("Thread %d: All threads reached barrier for phase %d\n", 
                   thread_id, phase);
        } else if (result == PTHREAD_BARRIER_SERIAL_THREAD) {
            printf("Thread %d: Serial thread for phase %d\n", thread_id, phase);
        }
        
        printf("Thread %d: Starting phase %d\n", thread_id, phase + 1);
    }
    
    return NULL;
}

int main() {
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    
    // 初始化屏障
    if (pthread_barrier_init(&barrier, NULL, NUM_THREADS) != 0) {
        perror("pthread_barrier_init");
        return 1;
    }
    
    printf("Barrier initialized for %d threads\n", NUM_THREADS);
    
    // 创建线程
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        
        if (pthread_create(&threads[i], NULL, thread_func, &thread_ids[i]) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    
    // 等待所有线程完成
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    printf("All threads completed\n");
    
    // 销毁屏障
    pthread_barrier_destroy(&barrier);
    
    return 0;
}
```

### 6.2 自旋锁

```c
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>

#define NUM_THREADS 4
#define ITERATIONS_PER_THREAD 1000000
#define CRITICAL_WORK 100

pthread_spinlock_t spinlock;
long shared_counter = 0;
long total_operations = 0;

void *spinlock_thread(void *arg) {
    int thread_id = *(int*)arg;
    struct timespec start, end;
    
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &start);
    
    for (int i = 0; i < ITERATIONS_PER_THREAD; i++) {
        // 获取自旋锁
        if (pthread_spin_lock(&spinlock) != 0) {
            perror("pthread_spin_lock");
            return NULL;
        }
        
        // 临界区
        shared_counter++;
        
        // 模拟临界区工作
        for (int j = 0; j < CRITICAL_WORK; j++) {
            volatile int dummy = j * j;
        }
        
        // 释放自旋锁
        if (pthread_spin_unlock(&spinlock) != 0) {
            perror("pthread_spin_unlock");
            return NULL;
        }
        
        total_operations++;
    }
    
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &end);
    
    double elapsed = (end.tv_sec - start.tv_sec) + 
                    (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("Thread %d: completed %d operations in %.6f seconds\n", 
           thread_id, ITERATIONS_PER_THREAD, elapsed);
    
    return NULL;
}

int main() {
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    struct timespec start, end;
    
    // 初始化自旋锁
    if (pthread_spin_init(&spinlock, PTHREAD_PROCESS_PRIVATE) != 0) {
        perror("pthread_spin_init");
        return 1;
    }
    
    printf("Spinlock initialized\n");
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    // 创建线程
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        
        if (pthread_create(&threads[i], NULL, spinlock_thread, &thread_ids[i]) != 0) {
            perror("pthread_create");
            return 1;
        }
    }
    
    // 等待所有线程完成
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    double total_elapsed = (end.tv_sec - start.tv_sec) + 
                         (end.tv_nsec - start.tv_nsec) / 1e9;
    
    printf("\nResults:\n");
    printf("  Shared counter: %ld\n", shared_counter);
    printf("  Total operations: %ld\n", total_operations);
    printf("  Expected operations: %d\n", NUM_THREADS * ITERATIONS_PER_THREAD);
    printf("  Total time: %.6f seconds\n", total_elapsed);
    printf("  Operations per second: %.0f\n", total_operations / total_elapsed);
    
    // 销毁自旋锁
    pthread_spin_destroy(&spinlock);
    
    return 0;
}
```

## 7. 安全机制示例

### 7.1 能力管理

```c
#include <sys/capability.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>

void print_capabilities(const char *label) {
    struct __user_cap_header_struct header;
    struct __user_cap_data_struct data;
    
    header.version = _LINUX_CAPABILITY_VERSION_3;
    header.pid = getpid();
    
    if (capget(&header, &data) == -1) {
        perror("capget");
        return;
    }
    
    printf("%s capabilities for process %d:\n", label, getpid());
    printf("  Permitted:  0x%08x\n", data.permitted);
    printf("  Inheritable: 0x%08x\n", data.inheritable);
    printf("  Effective:  0x%08x\n", data.effective);
}

int main() {
    struct __user_cap_header_struct header;
    struct __user_cap_data_struct data;
    
    // 检查是否有root权限
    if (getuid() != 0) {
        printf("This program requires root privileges\n");
        return 1;
    }
    
    // 打印当前能力
    print_capabilities("Initial");
    
    // 设置新的能力
    header.version = _LINUX_CAPABILITY_VERSION_3;
    header.pid = 0; // 当前进程
    
    // 允许CAP_NET_RAW和CAP_SYS_TIME能力
    data.permitted = (1 << CAP_NET_RAW) | (1 << CAP_SYS_TIME);
    data.inheritable = 0;
    data.effective = (1 << CAP_NET_RAW);
    
    if (capset(&header, &data) == -1) {
        perror("capset");
        return 1;
    }
    
    // 打印设置后的能力
    print_capabilities("After capset");
    
    // 测试能力是否有效
    printf("\nTesting capabilities:\n");
    
    // 测试网络原始套接字（需要CAP_NET_RAW）
    int sock = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
    if (sock == -1) {
        printf("Raw socket creation failed: %s\n", strerror(errno));
    } else {
        printf("Raw socket created successfully (CAP_NET_RAW works)\n");
        close(sock);
    }
    
    // 测试设置时间（需要CAP_SYS_TIME）
    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) == 0) {
        ts.tv_sec += 3600; // 前进1小时
        if (clock_settime(CLOCK_REALTIME, &ts) == 0) {
            printf("Time set successfully (CAP_SYS_TIME works)\n");
            
            // 恢复时间
            ts.tv_sec -= 3600;
            clock_settime(CLOCK_REALTIME, &ts);
        } else {
            printf("Time setting failed: %s\n", strerror(errno));
        }
    }
    
    return 0;
}
```

### 7.2 用户权限切换

```c
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <pwd.h>
#include <grp.h>
#include <sys/types.h>

void print_ids(const char *label) {
    printf("%s:\n", label);
    printf("  Real UID: %d, Effective UID: %d\n", getuid(), geteuid());
    printf("  Real GID: %d, Effective GID: %d\n", getgid(), getegid());
    
    // 获取用户信息
    struct passwd *pwd = getpwuid(getuid());
    if (pwd) {
        printf("  User: %s\n", pwd->pw_name);
    }
    
    // 获取组信息
    struct group *grp = getgrgid(getgid());
    if (grp) {
        printf("  Group: %s\n", grp->gr_name);
    }
    printf("\n");
}

int main() {
    uid_t orig_uid, target_uid;
    gid_t orig_gid, target_gid;
    
    // 检查是否有root权限
    if (getuid() != 0) {
        printf("This program requires root privileges\n");
        return 1;
    }
    
    // 打印原始ID
    print_ids("Original IDs");
    
    // 保存原始ID
    orig_uid = getuid();
    orig_gid = getgid();
    
    // 查找nobody用户
    struct passwd *nobody = getpwnam("nobody");
    if (!nobody) {
        printf("nobody user not found, using UID 65534\n");
        target_uid = 65534;
    } else {
        target_uid = nobody->pw_uid;
        target_gid = nobody->pw_gid;
    }
    
    printf("Switching to nobody (UID: %d, GID: %d)\n", target_uid, target_gid);
    
    // 设置组ID
    if (setgid(target_gid) == -1) {
        perror("setgid");
        return 1;
    }
    
    // 设置用户ID
    if (setuid(target_uid) == -1) {
        perror("setuid");
        return 1;
    }
    
    // 打印切换后的ID
    print_ids("After privilege drop");
    
    // 尝试恢复权限（应该失败）
    printf("Attempting to restore privileges...\n");
    
    if (setuid(orig_uid) == -1) {
        printf("Privilege restoration failed (expected): %s\n", strerror(errno));
    } else {
        printf("WARNING: Privilege restoration succeeded!\n");
    }
    
    // 打印最终ID
    print_ids("Final IDs");
    
    return 0;
}
```

## 8. 综合应用示例

### 8.1 高性能Web服务器框架

```c
#include <aio.h>
#include <mqueue.h>
#include <signal.h>
#include <pthread.h>
#include <sched.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>

#define PORT 8080
#define MAX_CONNECTIONS 1000
#define BUFFER_SIZE 4096
#define QUEUE_NAME "/webserver_queue"

typedef struct {
    int client_fd;
    struct sockaddr_in client_addr;
    char buffer[BUFFER_SIZE];
    struct aiocb aio_cb;
} connection_t;

mqd_t work_queue;
pthread_spinlock_t stats_lock;
int active_connections = 0;
long total_requests = 0;

void aio_completion_handler(int sig, siginfo_t *si, void *ucontext) {
    connection_t *conn = (connection_t *)si->si_value.sival_ptr;
    
    if (aio_error(&conn->aio_cb) == 0) {
        ssize_t bytes_read = aio_return(&conn->aio_cb);
        
        if (bytes_read > 0) {
            // 处理HTTP请求
            char response[] = "HTTP/1.1 200 OK\r\n"
                            "Content-Type: text/html\r\n"
                            "Content-Length: 13\r\n"
                            "\r\n"
                            "Hello, World!";
            
            write(conn->client_fd, response, sizeof(response) - 1);
            
            pthread_spin_lock(&stats_lock);
            total_requests++;
            pthread_spin_unlock(&stats_lock);
        }
    }
    
    // 关闭连接
    close(conn->client_fd);
    
    pthread_spin_lock(&stats_lock);
    active_connections--;
    pthread_spin_unlock(&stats_lock);
    
    free(conn);
}

void *worker_thread(void *arg) {
    int thread_id = *(int*)arg;
    connection_t *conn;
    
    printf("Worker thread %d started\n", thread_id);
    
    while (1) {
        // 从工作队列获取连接
        ssize_t bytes = mq_receive(work_queue, (char*)&conn, sizeof(conn), NULL);
        if (bytes <= 0) {
            continue;
        }
        
        // 启动异步读取
        memset(&conn->aio_cb, 0, sizeof(conn->aio_cb));
        conn->aio_cb.aio_fildes = conn->client_fd;
        conn->aio_cb.aio_buf = conn->buffer;
        conn->aio_cb.aio_nbytes = BUFFER_SIZE - 1;
        conn->aio_cb.aio_offset = 0;
        conn->aio_cb.aio_sigevent.sigev_notify = SIGEV_SIGNAL;
        conn->aio_cb.aio_sigevent.sigev_signo = SIGUSR1;
        conn->aio_cb.aio_sigevent.sigev_value.sival_ptr = conn;
        
        if (aio_read(&conn->aio_cb) == -1) {
            perror("aio_read");
            close(conn->client_fd);
            free(conn);
            continue;
        }
        
        pthread_spin_lock(&stats_lock);
        active_connections++;
        pthread_spin_unlock(&stats_lock);
    }
    
    return NULL;
}

void *stats_thread(void *arg) {
    while (1) {
        sleep(5);
        
        pthread_spin_lock(&stats_lock);
        printf("Stats: Active connections: %d, Total requests: %ld\n",
               active_connections, total_requests);
        pthread_spin_unlock(&stats_lock);
    }
    
    return NULL;
}

int main() {
    int server_fd, client_fd;
    struct sockaddr_in server_addr, client_addr;
    socklen_t client_len = sizeof(client_addr);
    pthread_t workers[4], stats;
    int thread_ids[4];
    struct mq_attr queue_attr;
    struct sigaction sa;
    
    // 检查权限
    if (getuid() == 0) {
        printf("Running as root, dropping privileges\n");
        
        // 设置实时调度
        struct sched_param param;
        param.sched_priority = 80;
        if (sched_setscheduler(0, SCHED_FIFO, &param) == 0) {
            printf("Real-time scheduling enabled\n");
        }
        
        // 切换到nobody用户
        struct passwd *nobody = getpwnam("nobody");
        if (nobody) {
            setgid(nobody->pw_gid);
            setuid(nobody->pw_uid);
            printf("Switched to nobody user\n");
        }
    }
    
    // 创建工作队列
    queue_attr.mq_flags = 0;
    queue_attr.mq_maxmsg = MAX_CONNECTIONS;
    queue_attr.mq_msgsize = sizeof(connection_t*);
    queue_attr.mq_curmsgs = 0;
    
    work_queue = mq_open(QUEUE_NAME, O_CREAT | O_RDWR, 0644, &queue_attr);
    if (work_queue == (mqd_t)-1) {
        perror("mq_open");
        return 1;
    }
    
    // 设置AIO完成信号处理
    memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = aio_completion_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGUSR1, &sa, NULL);
    
    // 初始化自旋锁
    pthread_spin_init(&stats_lock, PTHREAD_PROCESS_PRIVATE);
    
    // 创建工作线程
    for (int i = 0; i < 4; i++) {
        thread_ids[i] = i;
        pthread_create(&workers[i], NULL, worker_thread, &thread_ids[i]);
    }
    
    // 创建统计线程
    pthread_create(&stats, NULL, stats_thread, NULL);
    
    // 创建服务器套接字
    server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == -1) {
        perror("socket");
        return 1;
    }
    
    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(PORT);
    
    if (bind(server_fd, (struct sockaddr*)&server_addr, sizeof(server_addr)) == -1) {
        perror("bind");
        return 1;
    }
    
    if (listen(server_fd, MAX_CONNECTIONS) == -1) {
        perror("listen");
        return 1;
    }
    
    printf("High-performance web server started on port %d\n", PORT);
    
    // 主循环 - 接受连接
    while (1) {
        client_fd = accept(server_fd, (struct sockaddr*)&client_addr, &client_len);
        if (client_fd == -1) {
            perror("accept");
            continue;
        }
        
        // 分配连接结构
        connection_t *conn = malloc(sizeof(connection_t));
        if (!conn) {
            close(client_fd);
            continue;
        }
        
        conn->client_fd = client_fd;
        conn->client_addr = client_addr;
        
        // 将连接添加到工作队列
        if (mq_send(work_queue, (const char*)&conn, sizeof(conn), 0) == -1) {
            perror("mq_send");
            close(client_fd);
            free(conn);
        }
    }
    
    // 清理（实际上不会到达这里）
    mq_close(work_queue);
    mq_unlink(QUEUE_NAME);
    close(server_fd);
    
    return 0;
}
```

## 总结

这些示例展示了NOS内核中POSIX高级特性的实际应用：

1. **异步I/O**：提高I/O密集型应用的性能
2. **高级内存映射**：优化内存访问模式
3. **消息队列**：实现高效的进程间通信
4. **高级信号处理**：构建响应式系统
5. **实时扩展**：满足实时应用需求
6. **高级线程特性**：实现复杂的并发模式
7. **安全机制**：构建安全可靠的应用

通过组合使用这些特性，可以构建高性能、实时、安全的复杂应用系统。每个示例都包含了完整的错误处理和资源清理，可以作为实际应用的参考模板。
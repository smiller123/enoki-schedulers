#include <sched.h>
#include <stdio.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <linux/ghost.h>
#include <sys/mman.h>
#include <pthread.h>

struct queue {
	uint32_t offset;
	uint32_t capacity;
	uint32_t head;
	uint32_t tail;
};

struct send_msg {
	uint64_t pid;
	uint32_t prio0;
	uint32_t prio1;
	uint32_t prio2;
	uint32_t prio3;
	uint32_t prio4;
	uint32_t prio5;
	uint32_t prio6;
	uint32_t prio7;
};

struct rcv_msg {
	bool reclaim;
};

void enqueue(struct queue *q, struct send_msg msg) {
	struct send_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->head & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	ptr->pid = msg.pid;
	ptr->prio0 = msg.prio0;
	ptr->prio1 = msg.prio1;
	ptr->prio2 = msg.prio2;
	ptr->prio3 = msg.prio3;
	ptr->prio4 = msg.prio4;
	ptr->prio5 = msg.prio5;
	ptr->prio6 = msg.prio6;
	ptr->prio7 = msg.prio7;
	q->head += 1;
}

struct rcv_msg dequeue(struct queue *q) {
    struct rcv_msg msg;
	struct rcv_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->tail & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	q->tail += 1;
	msg.reclaim = ptr->reclaim;
	return msg;
}

void *thread(void *arg) {
    int fd = *(int *)arg;
    struct bento_ioc_create_queue create_queue;
    void *map_region;
    struct queue *q;
    int q_fd;
    struct rcv_msg msg;
    create_queue.elems = 4;
    create_queue.flags = 0;
    create_queue.mapsize = 0;
    q_fd = ioctl(fd, GHOST_IOC_CREATE_REV_QUEUE, (int32_t*) &create_queue);
    printf("mapsize %d\n", create_queue.mapsize);
    printf("q_fd %d\n", q_fd);

    map_region = mmap(0, create_queue.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, q_fd, 0);
    printf("errno %d\n", errno);
    printf("map_region %p\n", map_region);

    q = (struct queue *)map_region;
    printf("q ptr %d\n", q->offset);
    printf("q capacity %d\n", q->capacity);
    printf("q head %d\n", q->head);
    printf("q tail %d\n", q->tail);

    struct sched_param param;
    int pid_num = 0;
    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
    pthread_yield();
    printf("thread says hi\n");
    while(1) {
	if (q->head > q->tail) {
	    msg = dequeue(q);
	    printf("got msg %d\n", msg.reclaim);
	    pthread_yield();
	}
    }
    pthread_exit(arg);
    printf("thread says hi2\n");
}

main ()
{
    struct sched_param param;
    int pid_num = 0;
    int fd;
    struct bento_ioc_create_queue create_queue;
    struct bento_ioc_create_queue create_queue2;
    struct bento_ioc_enter_queue enter_queue;
    struct bento_ioc_enter_queue enter_queue2;
    void *map_region;
    void *map_region2;
    struct queue *q;
    struct queue *q2;
    int q_fd;
    int q_fd2;
    struct send_msg msg;
    struct rcv_msg msg2;

    create_queue.elems = 4;
    create_queue.flags = 0;
    create_queue.mapsize = 0;

    fd = open("/sys/fs/ghost/enclave_10/ctl", O_RDWR);
    printf("errno %d\n", errno);
    q_fd = ioctl(fd, GHOST_IOC_CREATE_QUEUE, (int32_t*) &create_queue);
    printf("mapsize %d\n", create_queue.mapsize);
    printf("q_fd %d\n", q_fd);

    map_region = mmap(0, create_queue.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, q_fd, 0);
    printf("errno %d\n", errno);
    printf("map_region %p\n", map_region);

    q = (struct queue *)map_region;
    printf("q ptr %d\n", q->offset);
    printf("q capacity %d\n", q->capacity);
    printf("q head %d\n", q->head);
    printf("q tail %d\n", q->tail);

    msg.pid = (uint32_t) getpid();
    msg.prio0 = 1;
    msg.prio1 = 0;
    msg.prio2 = 0;
    msg.prio3 = 0;
    msg.prio4 = 0;
    msg.prio5 = 0;
    msg.prio6 = 0;
    msg.prio7 = 0;
    enqueue(q, msg);

    enter_queue.entries = 1;
    enter_queue.id = create_queue.id;
    ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue);

    create_queue2.elems = 4;
    create_queue2.flags = 0;
    create_queue2.mapsize = 0;

    q_fd2 = ioctl(fd, GHOST_IOC_CREATE_REV_QUEUE, (int32_t*) &create_queue2);
    printf("mapsize %d\n", create_queue2.mapsize);
    printf("q_fd %d\n", q_fd2);

    map_region2 = mmap(0, create_queue2.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, q_fd2, 0);
    printf("errno %d\n", errno);
    printf("map_region %p\n", map_region);

    q2 = (struct queue *)map_region2;
    printf("q ptr %d\n", q2->offset);
    printf("q capacity %d\n", q2->capacity);
    printf("q head %d\n", q2->head);
    printf("q tail %d\n", q2->tail);

    param.sched_priority = 0;
    pthread_t thid;
    pthread_t thid2;
    pthread_t thid3;
    pthread_t thid4;
    pthread_t thid5;
    pthread_t thid6;
    pthread_t thid7;
    pthread_t thid8;
    pthread_create(&thid, NULL, thread, &fd);
    pthread_create(&thid2, NULL, thread, &fd);
    printf("made thread\n");

    sleep(5);
    msg.pid = (uint32_t) getpid();
    msg.prio0 = 2;
    msg.prio1 = 0;
    msg.prio2 = 0;
    msg.prio3 = 0;
    msg.prio4 = 0;
    msg.prio5 = 0;
    msg.prio6 = 0;
    msg.prio7 = 0;
    enqueue(q, msg);

    enter_queue.entries = 1;
    enter_queue.id = create_queue.id;
    ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue);

    sleep(5);
    msg.pid = (uint32_t) getpid();
    msg.prio0 = 1;
    msg.prio1 = 0;
    msg.prio2 = 0;
    msg.prio3 = 0;
    msg.prio4 = 0;
    msg.prio5 = 0;
    msg.prio6 = 0;
    msg.prio7 = 0;
    enqueue(q, msg);

    enter_queue.entries = 1;
    enter_queue.id = create_queue.id;
    ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue);

    sleep(5);

    void *ret;
    pthread_join(thid, &ret);
    pthread_join(thid2, &ret);
    printf("joined thread\n");
}

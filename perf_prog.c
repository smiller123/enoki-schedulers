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

struct queue {
	uint32_t offset;
	uint32_t capacity;
	uint32_t head;
	uint32_t tail;
};

struct sched_msg {
	uint32_t val;
};

void enqueue(struct queue *q, struct sched_msg msg) {
	struct sched_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->head & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	ptr->val = msg.val;
	printf("ptr val %d\n", ptr->val);
	q->head += 1;
    //msg = (struct sched_msg *)((void *)map_region + q->offset);
}

struct sched_msg dequeue(struct queue *q) {
    struct sched_msg msg;
	struct sched_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->tail & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	q->tail += 1;
	printf("ptr val %d\n", ptr->val);
	msg.val = ptr->val;
	return msg;
    //msg = (struct sched_msg *)((void *)map_region + q->offset);
}

main ()
{
    struct sched_param param;
    int pid_num = 0;
    char command[100];
    strcpy(command, "perf bench sched pipe");
    //int fd;
    //struct bento_ioc_create_queue create_queue;
    //struct bento_ioc_create_queue create_queue2;
    //struct bento_ioc_enter_queue enter_queue;
    //struct bento_ioc_enter_queue enter_queue2;
    //void *map_region;
    //void *map_region2;
    //struct queue *q;
    //struct queue *q2;
    //int q_fd;
    //int q_fd2;
    //struct sched_msg msg;
    //struct sched_msg msg2;

    //create_queue.elems = 4;
    //create_queue.flags = 0;
    //create_queue.mapsize = 0;

    //fd = open("/sys/fs/ghost/enclave_10/ctl", O_RDWR);
    //printf("errno %d\n", errno);
    //q_fd = ioctl(fd, GHOST_IOC_CREATE_QUEUE, (int32_t*) &create_queue);
    ////q_fd = ioctl(fd, GHOST_IOC_CREATE_REV_QUEUE, (int32_t*) &create_queue);
    //printf("mapsize %d\n", create_queue.mapsize);
    //printf("q_fd %d\n", q_fd);

    //map_region = mmap(0, create_queue.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, q_fd, 0);
    //printf("errno %d\n", errno);
    //printf("map_region %p\n", map_region);

    //q = (struct queue *)map_region;
    //printf("q ptr %d\n", q->offset);
    //printf("q capacity %d\n", q->capacity);
    //printf("q head %d\n", q->head);
    //printf("q tail %d\n", q->tail);
    //////msg = (struct sched_msg *)((void *)map_region + q->offset);
    //////msg->val = 10;
    //////q->head += 1;
    ////while (q->head == 0) {
    ////	sleep(1);
    ////}
    ////msg = dequeue(q);
    ////printf("msg val %d\n", msg.val);

    //msg.val = 10;
    //enqueue(q, msg);
    //msg.val = 20;
    //enqueue(q, msg);
    //msg.val = 30;
    //enqueue(q, msg);

    //enter_queue.entries = 3;
    //enter_queue.id = create_queue.id;
    //ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue);

    //create_queue2.elems = 4;
    //create_queue2.flags = 0;
    //create_queue2.mapsize = 0;

    //q_fd2 = ioctl(fd, GHOST_IOC_CREATE_QUEUE, (int32_t*) &create_queue2);
    ////q_fd = ioctl(fd, GHOST_IOC_CREATE_REV_QUEUE, (int32_t*) &create_queue);
    //printf("mapsize %d\n", create_queue2.mapsize);
    //printf("q_fd %d\n", q_fd2);

    //map_region2 = mmap(0, create_queue2.mapsize, PROT_READ | PROT_WRITE, MAP_SHARED, q_fd2, 0);
    //printf("errno %d\n", errno);
    //printf("map_region %p\n", map_region);

    //q2 = (struct queue *)map_region2;
    //printf("q ptr %d\n", q2->offset);
    //printf("q capacity %d\n", q2->capacity);
    //printf("q head %d\n", q2->head);
    //printf("q tail %d\n", q2->tail);
    //////msg = (struct sched_msg *)((void *)map_region + q->offset);
    //////msg->val = 10;
    //////q->head += 1;
    ////while (q->head == 0) {
    ////	sleep(1);
    ////}
    ////msg = dequeue(q);
    ////printf("msg val %d\n", msg.val);

    //msg2.val = 102;
    //enqueue(q2, msg2);
    //msg2.val = 202;
    //enqueue(q2, msg2);
    //msg2.val = 302;
    //enqueue(q2, msg2);

    //enter_queue2.entries = 3;
    //enter_queue2.id = create_queue2.id;
    //ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue2);
    ////ioctl(fd, EKIBEN_IOC_SEND_HINT, (int32_t*) &msg);
    //close(q_fd2);
    //close(q_fd);
    //close(fd);

    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
    system(command);
}

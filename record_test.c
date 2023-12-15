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

char *dequeue(struct queue *q) {
	char *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->tail & (q->capacity - 1);
	ptr += index * 256;
	printf("ptr curr %p\n", ptr);
	printf("ptr val %s\n", ptr);
	q->tail += 1;
	return ptr;
}

main ()
{
    struct sched_param param;
    int pid_num = 0;
    char command[100];
    strcpy(command, "perf bench sched pipe");
    int fd;
    struct bento_ioc_create_queue create_queue;
    void *map_region;
    struct queue *q;
    int q_fd;

    create_queue.elems = 16;
    create_queue.flags = 0;
    create_queue.mapsize = 0;

    fd = open("/sys/fs/ghost/enclave_10/ctl", O_RDWR);
    printf("errno %d\n", errno);
    q_fd = ioctl(fd, GHOST_IOC_CREATE_RECORD, (int32_t*) &create_queue);
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
    close(q_fd);
    close(fd);

    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
}

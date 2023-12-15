#define _GNU_SOURCE
#include <linux/sched.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
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

int child_func() {
	printf("Hello from child!\n");
	printf("Child PID: %ld\n", (long)getpid());
	return 0;
}

struct queue {
	uint32_t offset;
	uint32_t capacity;
	uint32_t head;
	uint32_t tail;
};

struct sched_msg {
	uint32_t tid;
	uint32_t hint;
};

void enqueue(struct queue *q, struct sched_msg msg) {
	struct sched_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->head & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	ptr->tid = msg.tid;
	ptr->hint = msg.hint;

	printf("ptr tid %d, ptr hint %d \n", ptr->tid, ptr->hint);
	q->head += 1;
}

struct sched_msg dequeue(struct queue *q) {
    struct sched_msg msg;
	struct sched_msg *ptr = (void *)q + q->offset;
	printf("ptr start %p\n", ptr);
	uint32_t index = q->tail & (q->capacity - 1);
	ptr += index;
	printf("ptr curr %p\n", ptr);
	q->tail += 1;
	printf("ptr tid %d, ptr hint %d \n", ptr->tid, ptr->hint);
	msg.tid = ptr->tid;
	msg.hint = ptr->hint;
	return msg;
}



int main(int argc, char *argv[]) {
	const int STACK_SIZE = 65536;
	char *stack;
	char *stack_top;
	pid_t child_pid;
	int flags = 0;

	pid_t p1_tids[1];
	pid_t p2_tids[1];

	// TODO: if this pid is reserved, change it out
	p1_tids[0] = 22000;
	p2_tids[0] = 23000;



	char command[100];
    	strcpy(command, "perf bench sched pipe");
    	int fd;
    	struct bento_ioc_create_queue create_queue;
    	struct bento_ioc_enter_queue enter_queue;
    	void *map_region;
    	struct queue *q;
    	int q_fd;
    	struct sched_msg msg;

    	create_queue.elems = 2;
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
    	msg = *((struct sched_msg *)((void *)map_region + q->offset));
    	printf("msg tid %d, msg hint %d\n", msg.tid, msg.hint);
	
	msg.tid = (uint32_t) p1_tids[0];
    	msg.hint = 2;
    	enqueue(q, msg);
    	msg.tid = (uint32_t) p2_tids[0];
	msg.hint = 2;
    	enqueue(q, msg);

    	enter_queue.entries = 2;
    	ioctl(fd, GHOST_IOC_ENTER_QUEUE, (int32_t*) &enter_queue);
    	ioctl(fd, EKIBEN_IOC_SEND_HINT, (int32_t*) &msg);
    	close(q_fd);
    	close(fd);

	struct sched_param param;
	int pid_num = 0;

	// need to choose which scheduler we are using, this will select
	// the data_aware_sched
	param.sched_priority = 0;
	sched_setscheduler(pid_num, 10, &param);

	struct clone_args p1_args = {
		.flags = flags,
		.pidfd = NULL,
		.child_tid = NULL,
		.parent_tid = NULL,
		.exit_signal = SIGCHLD,
		.stack = NULL,
		.stack_size = 0,
		.tls = NULL,
		.set_tid = &p1_tids, // do we need to worry about this cast?
		.set_tid_size = 1,
		.cgroup = NULL
	};

	struct clone_args p2_args = {
		.flags = flags,
		.pidfd = NULL,
		.child_tid = NULL,
		.parent_tid = NULL,
		.exit_signal = SIGCHLD,
		.stack = NULL,
		.stack_size = 0,
		.tls = NULL,
		.set_tid = &p2_tids, // do we need to worry about this cast?
		.set_tid_size = 1,
		.cgroup = NULL
	};

        // Call clone to create child
        pid_t child1_pid = syscall(SYS_clone3, &p1_args, sizeof(p1_args));
	if (child1_pid == -1) {
		perror("clone3");
		exit(EXIT_FAILURE);	
	} else if (child1_pid == 0) {
		child_func();
		return 0;
	}

        pid_t child2_pid = syscall(SYS_clone3, &p2_args, sizeof(p2_args));
	if (child2_pid == -1) {
		perror("clone3");
		exit(EXIT_FAILURE);	
	} else if (child2_pid == 0) {
		child_func();
		return 0;
	}
	printf("Bruh\n");

	printf("Parent PID: %d, child1's PID: %d\n", getpid(), child1_pid);

	printf("Parent PID: %d, child2's PID: %d\n", getpid(), child2_pid);


	if (waitpid(child1_pid, NULL, 0) == -1) {
		perror("waitpid");
		exit(EXIT_FAILURE);
	}

	if (waitpid(child2_pid, NULL, 0) == -1) {
		perror("waitpid");
		exit(EXIT_FAILURE);
	}

	printf("Parent done\n");

	return 0;
}

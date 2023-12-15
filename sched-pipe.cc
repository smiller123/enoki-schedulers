// SPDX-License-Identifier: GPL-2.0
/*
 *
 * sched-pipe.c
 *
 * pipe: Benchmark for pipe()
 *
 * Based on pipe-test-1m.c by Ingo Molnar <mingo@redhat.com>
 *  http://people.redhat.com/mingo/cfs-scheduler/tools/pipe-test-1m.c
 * Ported to perf by Hitoshi Mitake <mitake@dcl.info.waseda.ac.jp>
 */
#include "Arachne/Arachne.h"

#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <sys/wait.h>
#include <string.h>
#include <errno.h>
#include <assert.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/syscall.h>
#include <iostream>

#include <pthread.h>

struct thread_data {
	int			nr;
	int			pipe_read;
	int			pipe_write;
	pthread_t		pthread;
	Arachne::ThreadId	tid;
	Arachne::ThreadId	*other_tid;
};

#define USEC_PER_SEC 1000000
#define USEC_PER_MSEC 1000

#define LOOPS_DEFAULT 1000000
static	int			loops = LOOPS_DEFAULT;

/* Use processes by default: */
static bool			threaded;

static const char * const bench_sched_pipe_usage[] = {
	"perf bench sched pipe <options>",
	NULL
};

static void *worker_thread(void *__tdata)
{
	struct thread_data *td = (struct thread_data *)__tdata;
	int m = 0, i;
	int ret;
	while (td->other_tid == NULL) {}

	for (i = 0; i < loops; i++) {
		if (!td->nr) {
			Arachne::block();
			Arachne::signal(*td->other_tid);
		} else {
			Arachne::signal(*td->other_tid);
			Arachne::block();
		}
	}

	return NULL;
}

int bench_sched_pipe(int argc, const char **argv)
{
	struct thread_data threads[2], *td, *td1, *td2;
	int pipe_1[2], pipe_2[2];
	struct timeval start, stop, diff;
	unsigned long long result_usec = 0;
	int nr_threads = 2;
	int t;

	/*
	 * why does "ret" exist?
	 * discarding returned value of read(), write()
	 * causes error in building environment for perf
	 */
	int ret, wait_stat;
	pid_t pid, retpid;

	gettimeofday(&start, NULL);

	for (t = 0; t < nr_threads; t++) {
		td = threads + t;

		td->nr = t;
		td->other_tid = NULL;

		if (t == 0) {
			td->pipe_read = pipe_1[0];
			td->pipe_write = pipe_2[1];
		} else {
			td->pipe_write = pipe_1[1];
			td->pipe_read = pipe_2[0];
		}
	}


	threaded = true;
	if (threaded) {

		for (t = 0; t < nr_threads; t++) {
			td = threads + t;

			td->tid = Arachne::createThread(worker_thread, td);
		}
		td1 = threads + 0;
		td2 = threads + 1;
		td1->other_tid = &td2->tid;
		td2->other_tid = &td1->tid;

		for (t = 0; t < nr_threads; t++) {
			td = threads + t;
			Arachne::join(td->tid);
		}

	} else {
		pid = fork();
		assert(pid >= 0);

		if (!pid) {
			worker_thread(threads + 0);
			exit(0);
		} else {
			worker_thread(threads + 1);
		}

		retpid = waitpid(pid, &wait_stat, 0);
		assert((retpid == pid) && WIFEXITED(wait_stat));
	}

	gettimeofday(&stop, NULL);
	timersub(&stop, &start, &diff);

	printf("# Executed %d pipe operations between two %s\n\n",
		loops, threaded ? "threads" : "processes");

	result_usec = diff.tv_sec * USEC_PER_SEC;
	result_usec += diff.tv_usec;

	printf(" %14s: %lu.%03lu [sec]\n\n", "Total time",
	       diff.tv_sec,
	       (unsigned long) (diff.tv_usec / USEC_PER_MSEC));

	printf(" %14lf usecs/op\n",
	       (double)result_usec / (double)loops);
	printf(" %14d ops/sec\n",
	       (int)((double)loops /
		     ((double)result_usec / (double)USEC_PER_SEC)));
	return 0;
}

void AppMainWrapper(int argc, const char** argv) {
    bench_sched_pipe(argc, argv);
    Arachne::shutDown();
}
int main(int argc, const char** argv){
    Arachne::init(&argc, argv);
    Arachne::createThread(&AppMainWrapper, argc, argv);
    Arachne::waitForTermination();
}

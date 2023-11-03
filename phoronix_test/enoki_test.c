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
main ()
{
    struct sched_param param;
    int pid_num = 0;
    char command[400];
    strcpy(command, "FORCE_TIMES_TO_RUN=3 OUTPUT_FILE=enoki_tests_output TEST_RESULTS_NAME=enoki_eval TEST_RESULTS_IDENTIFIER=enoki_eval TEST_RESULTS_DESCRIPTION=enoki_eval ../phoronix-test-suite/phoronix-test-suite benchmark smiller/enoki-tests");

    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
    system(command);
}

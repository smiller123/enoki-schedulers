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
    char command[400];
    strcpy(command, "FORCE_TIMES_TO_RUN=3 OUTPUT_FILE=cfs_tests_output TEST_RESULTS_NAME=cfs_eval TEST_RESULTS_IDENTIFIER=cfs_eval TEST_RESULTS_DESCRIPTION=cfs_eval ../phoronix-test-suite/phoronix-test-suite benchmark enoki-tests");

    system(command);
}

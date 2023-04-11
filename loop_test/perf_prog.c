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
    char command[100];
    //strcpy(command, "cargo run");
    strcpy(command, "./target/debug/loop_test");

    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
    system(command);
}

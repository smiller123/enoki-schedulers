#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
main ()
{
    struct sched_param param;
    int pid_num = 0;
    char command[100];
    strcpy(command, "perf bench sched messaging");

    param.sched_priority = 0;
    sched_setscheduler(pid_num, 10, &param);
    system(command);
    //while (1) 
    //  ;
}

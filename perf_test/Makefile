all: enoki_test cfs_test

enoki_test: enoki_pipe_test.c
	gcc -o enoki_test enoki_pipe_test.c

cfs_test: cfs_pipe_test.c
	gcc -o cfs_test cfs_pipe_test.c

clean:
	rm enoki_test
	rm cfs_test

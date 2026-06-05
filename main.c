/*
 * app-ukwasmtime: Unikraft WebAssembly demo application
 *
 * Loads precompiled .cwasm modules from the filesystem (initrd)
 * and runs them using the lib-ukwasmtime C API.
 */

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdint.h>

#include <ukwasmtime.h>

static uint8_t *read_file(const char *path, size_t *out_len)
{
	int fd = open(path, O_RDONLY);
	if (fd < 0) {
		printf("ERROR: cannot open %s\n", path);
		return NULL;
	}

	off_t size = lseek(fd, 0, SEEK_END);
	if (size < 0) {
		printf("ERROR: lseek failed on %s\n", path);
		close(fd);
		return NULL;
	}
	lseek(fd, 0, SEEK_SET);

	uint8_t *buf = malloc((size_t)size);
	if (!buf) {
		printf("ERROR: malloc(%ld) failed\n", (long)size);
		close(fd);
		return NULL;
	}

	size_t total = 0;
	while (total < (size_t)size) {
		ssize_t n = read(fd, buf + total, (size_t)size - total);
		if (n <= 0)
			break;
		total += (size_t)n;
	}
	close(fd);

	if (total != (size_t)size) {
		printf("ERROR: read %zu of %ld bytes from %s\n",
		       total, (long)size, path);
		free(buf);
		return NULL;
	}

	*out_len = total;
	return buf;
}

static void demo_hello_module(void)
{
	size_t len;
	uint8_t *data;
	void *engine, *module;

	printf("=== Running hello module ===\n");
	printf("This is from the C file\n");

	data = read_file("/hello.cwasm", &len);
	if (!data)
		return;
	printf("  read %zu bytes\n", len);

	engine = ukwasmtime_engine_create();
	if (!engine) {
		printf("ERROR: engine creation failed\n");
		free(data);
		return;
	}
	printf("  engine created\n");

	module = ukwasmtime_module_load(engine, data, len);
	free(data);
	if (!module) {
		printf("ERROR: module load failed\n");
		ukwasmtime_engine_destroy(engine);
		return;
	}
	printf("  module loaded\n");

	if (ukwasmtime_module_run(engine, module) == 0)
		printf("  hello module finished OK\n");
	else
		printf("ERROR: module execution failed\n");

	ukwasmtime_module_destroy(module);
	ukwasmtime_engine_destroy(engine);
}

static void demo_add_module(void)
{
	size_t len;
	uint8_t *data;
	void *engine, *module;
	int32_t result;
	int32_t a = 3, b = 4;

	printf("=== Running add module ===\n");

	data = read_file("/add.cwasm", &len);
	if (!data)
		return;

	engine = ukwasmtime_engine_create();
	if (!engine) {
		printf("ERROR: engine creation failed\n");
		free(data);
		return;
	}

	module = ukwasmtime_module_load(engine, data, len);
	free(data);
	if (!module) {
		printf("ERROR: module load failed\n");
		ukwasmtime_engine_destroy(engine);
		return;
	}

	if (ukwasmtime_module_call_ii_i(engine, module, "add",
					a, b, &result) == 0)
		printf("  add(%d, %d) = %d\n", a, b, result);
	else
		printf("ERROR: add call failed\n");

	ukwasmtime_module_destroy(module);
	ukwasmtime_engine_destroy(engine);
}

int main(int argc __attribute__((unused)),
	 char *argv[] __attribute__((unused)))
{
	printf("app-ukwasmtime: Unikraft WebAssembly demo\n");

	demo_hello_module();
	demo_add_module();

	printf("app-ukwasmtime: done\n");
	return 0;
}

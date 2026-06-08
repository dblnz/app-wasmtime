/*
 * app-ukwasmtime: Unikraft WebAssembly demo entry point.
 *
 * The application logic lives in the Rust staticlib (app/src/lib.rs), which
 * exposes uk_wasmtime_main(). This C main() provides the entry point that
 * overrides ukboot's weak main and pulls the Rust archive into the link.
 */

#include <stdio.h>

extern int uk_wasmtime_main(void);

int main(int argc, char *argv[])
{
	(void)argc;
	(void)argv;

	/*
	 * Force unbuffered stdout.
	 *
	 * musl latches stdout's buffering mode on the first write: if
	 * ioctl(1, TIOCGWINSZ) succeeds it is line-buffered, otherwise fully
	 * buffered. When a network device is attached, lwip prints its
	 * "<if>: Interface is up" message via printf during early lib-init,
	 * before posix-tty binds fd 1 to the ioctl-capable serial console.
	 * stdout then gets latched as fully buffered, and since
	 * uk_wasmtime_main() runs the HTTP server loop without returning, the
	 * buffered startup output is never flushed and is lost. Force
	 * unbuffered stdout so all output reaches the console immediately.
	 */
	setvbuf(stdout, NULL, _IONBF, 0);

	return uk_wasmtime_main();
}

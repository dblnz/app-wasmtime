/*
 * app-ukwasmtime: Unikraft WebAssembly demo entry point.
 *
 * The application logic lives in the Rust staticlib (app/src/lib.rs), which
 * exposes uk_wasmtime_main(). This C main() provides the entry point that
 * overrides ukboot's weak main and pulls the Rust archive into the link.
 */

extern int uk_wasmtime_main(void);

int main(int argc, char *argv[])
{
	(void)argc;
	(void)argv;

	return uk_wasmtime_main();
}

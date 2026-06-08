/*
 * app-faas-wasmtime: Unikraft FaaS WebAssembly runner
 *
 * Reads a config file (/config) from the initrd that specifies which
 * precompiled .cwasm to load, which function to call, and what arguments
 * to pass.  Outputs structured results prefixed with FAAS_RESULT: or
 * FAAS_ERROR: for machine parsing.
 *
 * Config format (key=value, one per line):
 *   type=module|component
 *   file=/function.cwasm
 *   function=add           (omit or set to _start for run-style)
 *   argc=2
 *   arg0=3
 *   arg1=4
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdint.h>

#include <ukwasmtime.h>

#define MAX_ARGS       8
#define MAX_RESULTS    4
#define MAX_LINE     256
#define MAX_PATH     128
#define MAX_FUNCNAME  64

struct faas_config {
	char     type[16];                /* "module" or "component" */
	char     file[MAX_PATH];          /* path to .cwasm in initrd */
	char     function[MAX_FUNCNAME];  /* export name, empty = _start */
	int      argc;
	int32_t  args[MAX_ARGS];
};

/* ------------------------------------------------------------------ */

static uint8_t *read_file(const char *path, size_t *out_len)
{
	int fd = open(path, O_RDONLY);
	if (fd < 0)
		return NULL;

	off_t size = lseek(fd, 0, SEEK_END);
	if (size < 0) {
		close(fd);
		return NULL;
	}
	lseek(fd, 0, SEEK_SET);

	uint8_t *buf = malloc((size_t)size);
	if (!buf) {
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
		free(buf);
		return NULL;
	}

	*out_len = total;
	return buf;
}

static int parse_config(const char *text, struct faas_config *cfg)
{
	memset(cfg, 0, sizeof(*cfg));

	const char *p = text;
	while (*p) {
		/* skip whitespace */
		while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n')
			p++;
		if (*p == '\0' || *p == '#')
			break;

		/* find key */
		const char *key = p;
		while (*p && *p != '=' && *p != '\n')
			p++;
		if (*p != '=')
			continue;

		size_t klen = (size_t)(p - key);
		p++; /* skip '=' */

		/* find value */
		const char *val = p;
		while (*p && *p != '\n' && *p != '\r')
			p++;
		size_t vlen = (size_t)(p - val);

		if (klen == 4 && memcmp(key, "type", 4) == 0) {
			if (vlen >= sizeof(cfg->type))
				vlen = sizeof(cfg->type) - 1;
			memcpy(cfg->type, val, vlen);
			cfg->type[vlen] = '\0';
		} else if (klen == 4 && memcmp(key, "file", 4) == 0) {
			if (vlen >= sizeof(cfg->file))
				vlen = sizeof(cfg->file) - 1;
			memcpy(cfg->file, val, vlen);
			cfg->file[vlen] = '\0';
		} else if (klen == 8 && memcmp(key, "function", 8) == 0) {
			if (vlen >= sizeof(cfg->function))
				vlen = sizeof(cfg->function) - 1;
			memcpy(cfg->function, val, vlen);
			cfg->function[vlen] = '\0';
		} else if (klen == 4 && memcmp(key, "argc", 4) == 0) {
			char tmp[16] = {0};
			if (vlen >= sizeof(tmp))
				vlen = sizeof(tmp) - 1;
			memcpy(tmp, val, vlen);
			cfg->argc = atoi(tmp);
			if (cfg->argc > MAX_ARGS)
				cfg->argc = MAX_ARGS;
		} else if (klen >= 4 && memcmp(key, "arg", 3) == 0) {
			/* arg0, arg1, ... */
			char idx_buf[4] = {0};
			size_t ilen = klen - 3;
			if (ilen >= sizeof(idx_buf))
				ilen = sizeof(idx_buf) - 1;
			memcpy(idx_buf, key + 3, ilen);
			int idx = atoi(idx_buf);
			if (idx >= 0 && idx < MAX_ARGS) {
				char tmp[16] = {0};
				if (vlen >= sizeof(tmp))
					vlen = sizeof(tmp) - 1;
				memcpy(tmp, val, vlen);
				cfg->args[idx] = (int32_t)atoi(tmp);
			}
		}
	}

	return (cfg->file[0] != '\0' && cfg->type[0] != '\0') ? 0 : -1;
}

/* ------------------------------------------------------------------ */

static int run_module(struct faas_config *cfg, uint8_t *data, size_t len)
{
	void *engine = ukwasmtime_engine_create();
	if (!engine) {
		printf("FAAS_ERROR:engine creation failed\n");
		return -1;
	}

	void *module = ukwasmtime_module_load(engine, data, len);
	if (!module) {
		printf("FAAS_ERROR:module load failed\n");
		ukwasmtime_engine_destroy(engine);
		return -1;
	}

	void *instance = ukwasmtime_instantiate_module(engine, module);
	if (!instance) {
		printf("FAAS_ERROR:module instantiate failed\n");
		ukwasmtime_module_destroy(module);
		ukwasmtime_engine_destroy(engine);
		return -1;
	}

	/* Empty function name means the run-style "_start" entry point. */
	const char *func = cfg->function[0] ? cfg->function : "_start";

	ukw_val_t args[MAX_ARGS];
	for (int i = 0; i < cfg->argc; i++)
		args[i] = UKW_VAL_I32(cfg->args[i]);

	ukw_val_t results[MAX_RESULTS];
	size_t nresults = 0;
	int rc = ukwasmtime_call(instance, func, args, (size_t)cfg->argc,
				 results, MAX_RESULTS, &nresults);
	if (rc == 0) {
		if (nresults >= 1 && results[0].kind == UKW_I32)
			printf("FAAS_RESULT:%d\n", results[0].of.i32);
		else
			printf("FAAS_RESULT:ok\n");
	} else {
		printf("FAAS_ERROR:call %s failed (rc=%d)\n", func, rc);
	}

	ukwasmtime_instance_free(instance);
	ukwasmtime_module_destroy(module);
	ukwasmtime_engine_destroy(engine);
	return rc;
}

static int run_component(struct faas_config *cfg, uint8_t *data, size_t len)
{
	void *engine = ukwasmtime_engine_create();
	if (!engine) {
		printf("FAAS_ERROR:engine creation failed\n");
		return -1;
	}

	void *component = ukwasmtime_component_load(engine, data, len);
	if (!component) {
		printf("FAAS_ERROR:component load failed\n");
		ukwasmtime_engine_destroy(engine);
		return -1;
	}

	void *instance = ukwasmtime_instantiate_component(engine, component);
	if (!instance) {
		printf("FAAS_ERROR:component instantiate failed\n");
		ukwasmtime_component_destroy(component);
		ukwasmtime_engine_destroy(engine);
		return -1;
	}

	ukw_val_t args[MAX_ARGS];
	for (int i = 0; i < cfg->argc; i++)
		args[i] = UKW_VAL_I32(cfg->args[i]);

	ukw_val_t results[MAX_RESULTS];
	size_t nresults = 0;
	int rc = ukwasmtime_call(instance, cfg->function, args,
				 (size_t)cfg->argc, results, MAX_RESULTS,
				 &nresults);
	if (rc == 0) {
		if (nresults >= 1 && results[0].kind == UKW_I32)
			printf("FAAS_RESULT:%d\n", results[0].of.i32);
		else
			printf("FAAS_RESULT:ok\n");
	} else {
		printf("FAAS_ERROR:call %s failed (rc=%d)\n",
		       cfg->function, rc);
	}

	ukwasmtime_instance_free(instance);
	ukwasmtime_component_destroy(component);
	ukwasmtime_engine_destroy(engine);
	return rc;
}

/* ------------------------------------------------------------------ */

int main(int argc, char *argv[])
{
	printf("app-faas-wasmtime: starting\n");

	/* 1. Read config */
	size_t cfg_len;
	uint8_t *cfg_data = read_file("/config", &cfg_len);
	if (!cfg_data) {
		printf("FAAS_ERROR:cannot read /config\n");
		return 1;
	}

	/* Null-terminate for string parsing */
	char *cfg_text = malloc(cfg_len + 1);
	if (!cfg_text) {
		printf("FAAS_ERROR:malloc failed\n");
		free(cfg_data);
		return 1;
	}
	memcpy(cfg_text, cfg_data, cfg_len);
	cfg_text[cfg_len] = '\0';
	free(cfg_data);

	struct faas_config cfg;
	if (parse_config(cfg_text, &cfg) != 0) {
		printf("FAAS_ERROR:invalid config\n");
		free(cfg_text);
		return 1;
	}
	free(cfg_text);

	printf("  type=%s file=%s func=%s argc=%d\n",
	       cfg.type, cfg.file,
	       cfg.function[0] ? cfg.function : "(default)",
	       cfg.argc);

	/* 2. Read the .cwasm file */
	size_t wasm_len;
	uint8_t *wasm_data = read_file(cfg.file, &wasm_len);
	if (!wasm_data) {
		printf("FAAS_ERROR:cannot read %s\n", cfg.file);
		return 1;
	}
	printf("  loaded %zu bytes from %s\n", wasm_len, cfg.file);

	/* 3. Dispatch based on type */
	int rc;
	if (strcmp(cfg.type, "module") == 0)
		rc = run_module(&cfg, wasm_data, wasm_len);
	else if (strcmp(cfg.type, "component") == 0)
		rc = run_component(&cfg, wasm_data, wasm_len);
	else {
		printf("FAAS_ERROR:unknown type '%s'\n", cfg.type);
		rc = -1;
	}

	free(wasm_data);
	printf("app-faas-wasmtime: done (rc=%d)\n", rc);
	return rc != 0 ? 1 : 0;
}

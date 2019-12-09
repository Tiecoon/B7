#include <stdio.h>
#include <stdbool.h>
#include <string.h>

#define PASSWORD "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
#define PASSWORD_SIZE 27

static volatile char password[PASSWORD_SIZE];

static void zero_buf(volatile char *buf, size_t size)
{
	for (size_t i = 0; i < size; i++) {
		buf[i] = 0;
	}
}

static bool mem_eq(const volatile char *s1,
		   const volatile char *s2,
		   size_t size)
{
	for (size_t i = 0; i < size; i++) {
		printf("%zd\n", i);
		if (*s1++ != *s2++) {
			return false;
		}
	}

	return true;
}

int main(void)
{
	zero_buf(password, PASSWORD_SIZE);

	if (mem_eq(password, PASSWORD, PASSWORD_SIZE)) {
		puts("Correct!");
	} else {
		puts("Wrong");
	}
}

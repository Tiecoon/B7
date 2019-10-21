#include <stdio.h>
#include <stdbool.h>

#define PASSWORD "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
#define PASSWORD_SIZE 27

static volatile char password[PASSWORD_SIZE];

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
	if (mem_eq(password, PASSWORD, PASSWORD_SIZE)) {
		puts("Correct!");
	} else {
		puts("Wrong");
	}
}

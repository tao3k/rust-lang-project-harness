#pragma once

#include <stddef.h>
#include <stdint.h>

#define NATIVE_ABI_VERSION 1
#define NATIVE_ABI_ID "native-abi-contract-surface.v1"

typedef struct NativeUtf8 {
  const uint8_t *ptr;
  size_t len;
} NativeUtf8;

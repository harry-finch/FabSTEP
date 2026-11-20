// Import the Win32 SDK before OCCT, then discard legacy memory-model and RPC
// macros that collide with names in the geometry implementation.
#pragma once
#ifdef _WIN32
#include <windows.h>
#include <rpcndr.h>
#undef near
#undef far
#undef small
#endif

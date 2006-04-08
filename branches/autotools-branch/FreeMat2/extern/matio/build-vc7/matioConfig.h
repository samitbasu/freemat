#include <io.h> /* Needed by MS non-standard _mktemp for ISO mktemp */
#define mktemp _mktemp
/* src/config.h.  Generated by configure.  */
#define HAVE_ZLIB 1
/* #undef HAVE_INTTYPES_H */
/* #undef HAVE_STDINT_H */
#define STDC_HEADERS 1

#define MATIO_PLATFORM "i686-pc-win32"
#define MATIO_MAJOR_VERSION 1
#define MATIO_MINOR_VERSION 1
#define MATIO_RELEASE_LEVEL 4

/* Define to how the fortran compiler mangles symbol names */
#define FC_FUNC(name,NAME) NAME
/* Define to how the fortran compiler mangles symbol names */
#define FC_FUNC_(name,NAME) NAME
/* Define to how the fortran  compiler mangles module symbol names */
#define FC_MODULE(module,MODULE,name,NAME) MODULE ## _mp_ ## NAME
/* Define to how the fortran  compiler mangles module symbol names */
#define FC_MODULE_(module,MODULE,name,NAME) MODULE ## _mp_ ## NAME


/* #undef LINUX */
#define WINNT 1
/* #undef SUN */

#define SIZEOF_DOUBLE 8
#define SIZEOF_FLOAT 4
#define SIZEOF_LONG 4
#define SIZEOF_INT 4
#define SIZEOF_SHORT 2
#define SIZEOF_CHAR 1

/* #undef HAVE_VA_COPY */
/* #undef HAVE___VA_COPY */
/* #undef HAVE_VSNPRINTF */
/* #undef HAVE_SNPRINTF */
/* #undef HAVE_VASPRINTF */
/* #undef HAVE_ASPRINTF */

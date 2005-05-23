TEMPLATE = lib

TARGET = AMD

CONFIG += staticlib

INCLUDEPATH += Include

HEADERS += Include/amd.h \
Source/amd_internal.h \

SOURCES += Source/amd_1.c \
Source/amd_2.c \
Source/amd_aat.c \
Source/amd_control.c \
Source/amd_defaults.c \
Source/amd_dump.c \
Source/amd_info.c \
Source/amd_order.c \
Source/amd_postorder.c \
Source/amd_post_tree.c \
Source/amd_preprocess.c \
Source/amd_valid.c 

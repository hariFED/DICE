cmake_minimum_required(VERSION 3.16)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(dice_firmware_tests)

# Add test components
idf_component_register(
    SRCS
        "test_entropy.c"
        "test_crypto.c"
        "test_commit_reveal.c"
    INCLUDE_DIRS
        "../main"
        "../components/dice_protocol"
    REQUIRES
        unity
        dice_protocol
)

cmake_minimum_required( VERSION 3.6 )

project( Test )

# Setup compile flags
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# CMAKE_PREFIX_PATH must contain the path to the toolkit/cmake subdirectory of
# the Houdini installation. See the "Compiling with CMake" section of the HDK
# documentation for more details, which describes several options for
# specifying this path.
list( APPEND CMAKE_PREFIX_PATH "$ENV{HFS}/toolkit/cmake" )

find_package( Houdini REQUIRED )

set( library_name test )

# Code generation for the embedded DS file in SOP_Test.C.
houdini_generate_proto_headers( FILES src/SOP_Test.C )

# Add a library and its source files.
add_library( ${library_name} SHARED
  src/SOP_Test.C
)

# Link against the Houdini libraries, and add required include directories and
# compile definitions.
target_link_libraries( ${library_name}
    PUBLIC Houdini
    PRIVATE ${Test_LIBRARIES})

if(WIN32)
    # Add Rust specific Windows system dependencies.
    target_link_libraries( ${library_name} PRIVATE Ws2_32 userenv )
endif(WIN32)

# Include ${CMAKE_CURRENT_BINARY_DIR} for the generated header.
target_include_directories( ${library_name} PRIVATE
    ${CMAKE_CURRENT_BINARY_DIR} ${Test_INCLUDE_DIRS}
)

# Sets several common target properties, such as the library's output directory.
# We don't want to actually install the test plugin so just leave it in the build directory.
houdini_configure_target( ${library_name} INSTDIR ${CMAKE_SOURCE_DIR}/build/ )

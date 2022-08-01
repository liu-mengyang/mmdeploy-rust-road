# 决战Rust bindgen——分析ffmpeg-next项目构建分析

## 0x01 背景

为mmDeploy加入Rust API，往往需要调用许多非Rust语言库，那么FFI（外部语言接口）绑定技术就是必备技能。

[`bindgen`](https://crates.io/crates/bindgen) 就是一个主流的自动生成C/C++ FFI绑定的Rust库和工具，[ffmpeg-next（也叫rust-ffmpeg）](https://github.com/zmwangx/rust-ffmpeg) 和[rust-ncnn](https://github.com/tpoisonooo/rust-ncnn)都基于其实现。前者包装了最知名的开源音视频处理库[FFmpeg](https://ffmpeg.org/)，后者包装了最知名的移动端神经网络推理库[ncnn](https://github.com/Tencent/ncnn)。

本文将以ffmpeg-next项目为例解构大型系统库调用绑定项目的 `build.rs` 编写逻辑和 `bindgen` 使用方法。

> 本文所有操作均在windows10系统上进行。
> 

## 0x02 Build过程

> 参考[Notes on buildingrust-ffmpeg Wiki](https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building)
> 

需要提前安装好LLVM，这里需要注意`llvm-config` 并不在LLVM windows二进制预编译包里，为此我们需要手动build LLVM，具体参考[llvm-project](https://github.com/llvm/llvm-project)。

需要提前安装好FFmpeg，直接下载预编译好的包即可，注意下载完整的预编译版本，需要包含 `lib` 和 `include` 文件夹，我用的[下载地址](https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2022-07-24-12-40/ffmpeg-n5.0.1-8-g54e0971edb-win64-gpl-shared-5.0.zip)，解压缩后将路径添加到环境变量 `FFMPEG_DIR`： `$env:FFMPEG_DIR="D:\Tools\ffmpeg\"`。

进行构建： `cargo build`

构建成功。

> 将如此复杂精细的构建过程表现得如此简单，即一行命令完成实在是令人感到舒畅，但这背后编写 `build.rs` 的人肯定没少掉头发
> 

## 0x03 逐行分析

推荐前置资料阅读：

- 关于Cargo的build脚本：[Build Scripts - The Cargo Book ](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- bindgen使用指南：[The `bindgen` User Guide](https://rust-lang.github.io/rust-bindgen/library-usage.html)

要想调用非Rust的外部库，我们需要书写一个 `build.rs` 程序在包的根目录，在这个程序中我们可以生成代码、编译代码以及链接系统库。为了链接系统库，需要在所开发crate的 `Cargo.toml` 中的 `[package]` section 中加入 `links` key。

对于Rust，通常会将链接了系统库的crate命名为 `*-sys` package，它负责链接系统库并提供low-level API。此外，还会有一个命名为 `*` 的package依赖于它并提供该系统库的high-level API，这样的分离设计方法是Rust的传统，且被认为是更加安全的开发模式。

那么对于[rust-ffmpeg](https://github.com/zmwangx/rust-ffmpeg)，它的 `build.rs` 写得非常简洁，没有使用bindgen，这是因为它直接依赖于 `ffmpeg-sys-next` 包提供的ffmpeg low-level API了。

为此，我们去探查 `ffmpeg-sys-next` 包的内部代码：[ffmpeg-sys-next](https://github.com/zmwangx/rust-ffmpeg-sys)。可以看到它的 `build.rs` 足足有1287行，下面进行逐行分析！

### Line 1~4：声明外部库

```rust
extern crate bindgen;
extern crate cc;
extern crate num_cpus;
extern crate pkg_config;
```

声明需要使用的外部库，这4个外部库常见于 `build.rs` 的编写，它们也被加入到了 `Cargo.toml` 的 `[build-dependencies]` section中。

4个库的简介：

- [bindgen](https://docs.rs/bindgen/latest/bindgen/)：用于自动生成C/C++库FFI绑定的代码。
- [cc](https://docs.rs/cc/latest/cc/)：用于编译自定义的C代码。
- [num_cpus](https://docs.rs/num_cpus/latest/num_cpus/)：提供一个 `get` 方法用于获取本地设备的cpu数目。
- [pkg_config](https://docs.rs/pkg-config/latest/pkg_config/)：用于调用系统中的 `pkg-config` 工具找到系统库位置。关于 `pkg-config`：[tldr-pkg-config](https://tldr.ostera.io/pkg-config)；

### Line 6~11：声明使用 `STL`库及其模块

```rust
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::str;
```

声明需要使用到的STL库及其模块等组件。对这几个库的简介：

- [`std::env`](https://doc.rust-lang.org/std/env/index.html) ：访问和操作程序的上下文环境。
- [`std::fs`](https://doc.rust-lang.org/std/fs/index.html) ：文件操作。
    - File struct：对于一个打开文件的引用。
- [`std::io`](https://doc.rust-lang.org/std/io/index.html) ：IO操作。
    - BufRead trait：控制带buffer的读操作形式。
    - BufReader struct：带buffer的reader。
    - Write trait：控制写操作形式。
- [`std::path`](https://doc.rust-lang.org/std/path/index.html) ：可实现跨平台的路径管理获取。
    - PathBuf struct：可修改路径，与String类似。
- [`std::process`](https://doc.rust-lang.org/std/process/index.html) ：开辟子进程分担工作。
    - Command struct：用于配置和开辟子进程。
- [`std::str`](https://doc.rust-lang.org/std/primitive.str.html) ：Rust的primitive类型之一，用于表示字符串。

### Line 13~15：声明使用 `bindgen`库 `callbacks`模块内部的Enums

```rust
use bindgen::callbacks::{
    EnumVariantCustomBehavior, EnumVariantValue, IntKind, MacroParsingBehavior, ParseCallbacks,
};
```

`bindgen` 库中的 `callbacks` 模块提供了一些类似回调函数的API封装在 `ParseCallbacks` trait中，在这里声明使用的还有该模块中的一些Enums：

- `EnumVariantCustomBehavior` ：表示一些自定义的变量行为。
- `EnumVariantValue` ：表示分配给变量的常量值。
- `IntKind` ：表示处理的整数类型。
- `MacroParsingBehavior` ：表示进行宏解析时的行为，实际就是是否忽略。

### Line 17~31：定义 `Library` struct

```rust
#[derive(Debug)]
struct Library {
    name: &'static str,
    is_feature: bool,
}

impl Library {
    fn feature_name(&self) -> Option<String> {
        if self.is_feature {
            Some("CARGO_FEATURE_".to_string() + &self.name.to_uppercase())
        } else {
            None
        }
    }
}
```

定义了一个 `Library` struct用与描述ffmpeg库信息。它有一个拥有静态生命周期的 str slice变量 `name` 表示库名称以及一个*bool*类型的 `is_feature` 变量表示是否为其配置feature。该struct有一个返回feature名的函数 `feature_name` ，如果调用的 `Library`实例要为其配置feature，就会返回其对应的环境变量名。为 `Library` struct获取默认的 `Debug` trait使其便于打印。

### Line 33~70：定义 `Library` slice常量存储ffmpeg库信息

```rust
static LIBRARIES: &[Library] = &[
    Library {
        name: "avcodec",
        is_feature: true,
    },
    Library {
        name: "avdevice",
        is_feature: true,
    },
    Library {
        name: "avfilter",
        is_feature: true,
    },
    Library {
        name: "avformat",
        is_feature: true,
    },
    Library {
        name: "avresample",
        is_feature: true,
    },
    Library {
        name: "avutil",
        is_feature: false,
    },
    Library {
        name: "postproc",
        is_feature: true,
    },
    Library {
        name: "swresample",
        is_feature: true,
    },
    Library {
        name: "swscale",
        is_feature: true,
    },
];
```

创建了内部元素类型为 `Library` 的slice常量 `LIBRARIES` 。共有9个Library得到创建，参考https://github.com/FFmpeg/FFmpeg项目可以知道，这对应到了ffmpeg的模块，需要注意的是， `avresample`  模块在 ffmpeg 4.0.0版本后就被弃用了，在这里仍然存留是为了兼容性考虑。下面对ffmpeg 8个模块进行简单介绍：

- libavcodec：编解码库；
- libavdevice：特殊设备上的格式组织/分解；
- libavfilter：基于图形的帧编辑库；
- libavformat：I/O及格式组织/分解；
- libavutil：通用组件库；
- libpostproc：后处理库；
- libswresample：音频再采样、格式转换与组织；
- libswscale：颜色转换与伸缩库；

### Line 72~131：定义 `CallBacks` struct并为其实现 `ParseCallbacks` trait

```rust
#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn int_macro(&self, _name: &str, value: i64) -> Option<IntKind> {
        let ch_layout_prefix = "AV_CH_";
        let codec_cap_prefix = "AV_CODEC_CAP_";
        let codec_flag_prefix = "AV_CODEC_FLAG_";
        let error_max_size = "AV_ERROR_MAX_STRING_SIZE";

        if value >= i64::min_value() as i64
            && value <= i64::max_value() as i64
            && _name.starts_with(ch_layout_prefix)
        {
            Some(IntKind::ULongLong)
        } else if value >= i32::min_value() as i64
            && value <= i32::max_value() as i64
            && (_name.starts_with(codec_cap_prefix) || _name.starts_with(codec_flag_prefix))
        {
            Some(IntKind::UInt)
        } else if _name == error_max_size {
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else if value >= i32::min_value() as i64 && value <= i32::max_value() as i64 {
            Some(IntKind::Int)
        } else {
            None
        }
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        let dummy_codec_id_prefix = "AV_CODEC_ID_FIRST_";
        if original_variant_name.starts_with(dummy_codec_id_prefix) {
            Some(EnumVariantCustomBehavior::Constify)
        } else {
            None
        }
    }

    // https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-388277405
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        use MacroParsingBehavior::*;

        match name {
            "FP_INFINITE" => Ignore,
            "FP_NAN" => Ignore,
            "FP_NORMAL" => Ignore,
            "FP_SUBNORMAL" => Ignore,
            "FP_ZERO" => Ignore,
            _ => Default,
        }
    }
}
```

- `int_macro` 函数用于返回给定宏的整型类型；在这里主要依赖ffmpeg相关宏的设计进行指定；
- `enum_variant_behavior` 函数用于返回给定enum变量是否要转换为一个常量；这里对一些在ffmpeg中就是固定常量的宏进行转换；
- `will_parse_macro` 函数用于控制和指定需要解析的宏；这里的实现过滤了若干数学表示宏，这是为了阻止因为enum和macro重名导致的bug：[enum and define with the same name collide · Issue #687 · rust-lang/rust-bindgen (github.com)](https://github.com/rust-lang/rust-bindgen/issues/687#issuecomment-388277405)。

### Line 133~144：工具函数 `version`

```rust
fn version() -> String {
    let major: u8 = env::var("CARGO_PKG_VERSION_MAJOR")
        .unwrap()
        .parse()
        .unwrap();
    let minor: u8 = env::var("CARGO_PKG_VERSION_MINOR")
        .unwrap()
        .parse()
        .unwrap();

    format!("{}.{}", major, minor)
}
```

根据Cargo的环境变量拿到包的版本号字符串。

### Line 146~160：三个路径相关工具函数

```rust
fn output() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn source() -> PathBuf {
    output().join(format!("ffmpeg-{}", version()))
}

fn search() -> PathBuf {
    let mut absolute = env::current_dir().unwrap();
    absolute.push(&output());
    absolute.push("dist");

    absolute
}
```

- `output` 函数拿到输出文件夹路径；
- `source` 函数拿到ffmpeg源代码路径；
- `search` 函数拿到输出目的的路径；

### Line 162~181 工具函数 `fetch`

```rust
fn fetch() -> io::Result<()> {
    let output_base_path = output();
    let clone_dest_dir = format!("ffmpeg-{}", version());
    let _ = std::fs::remove_dir_all(output_base_path.join(&clone_dest_dir));
    let status = Command::new("git")
        .current_dir(&output_base_path)
        .arg("clone")
        .arg("--depth=1")
        .arg("-b")
        .arg(format!("release/{}", version()))
        .arg("https://github.com/FFmpeg/FFmpeg")
        .arg(&clone_dest_dir)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "fetch failed"))
    }
}

```

可用于获取FFmpeg源代码。

### Line 183~190 工具函数 `switch`

```rust
fn switch(configure: &mut Command, feature: &str, name: &str) {
    let arg = if env::var("CARGO_FEATURE_".to_string() + feature).is_ok() {
        "--enable-"
    } else {
        "--disable-"
    };
    configure.arg(arg.to_string() + name);
}
```

可用于根据环境feature判断是否在配置中激活某一feature。

### Line 192~384 构建函数 `build`

用于从源文件编译库文件。

**Line 193**

```rust
let source_dir = source();
```

获取ffmpeg源文件目录。

**Line 196~199**

```rust
let configure_path = source_dir.join("configure");
assert!(configure_path.exists());
let mut configure = Command::new(&configure_path);
configure.current_dir(&source_dir);
```

创建configure命令，它将在系统上准备好软件的构建环境。

**Line 201**

```rust
configure.arg(format!("--prefix={}", search().to_string_lossy()));
```

为configure命令添加前缀参数，即输出目的目录。

**Line 203~222**

```rust
if env::var("TARGET").unwrap() != env::var("HOST").unwrap() {
    // Rust targets are subtly different than naming scheme for compiler prefixes.
    // The cc crate has the messy logic of guessing a working prefix,
    // and this is a messy way of reusing that logic.
    let cc = cc::Build::new();
    let compiler = cc.get_compiler();
    let compiler = compiler.path().file_stem().unwrap().to_str().unwrap();
    let suffix_pos = compiler.rfind('-').unwrap(); // cut off "-gcc"
    let prefix = compiler[0..suffix_pos].trim_end_matches("-wr"); // "wr-c++" compiler

    configure.arg(format!("--cross-prefix={}-", prefix));
    configure.arg(format!(
        "--arch={}",
        env::var("CARGO_CFG_TARGET_ARCH").unwrap()
    ));
    configure.arg(format!(
        "--target_os={}",
        env::var("CARGO_CFG_TARGET_OS").unwrap()
    ));
}
```

若编译环境较为混乱，使用 `cc` 工具进行环境查找，并为configure命令添加相应合适的参数。

**Line 225~231**

```rust
if env::var("DEBUG").is_ok() {
    configure.arg("--enable-debug");
    configure.arg("--disable-stripping");
} else {
    configure.arg("--disable-debug");
    configure.arg("--enable-stripping");
}
```

根据环境变量判断是否使用 `DEBUG` 编译模式。

**Line 234~235**

```rust
configure.arg("--enable-static");
configure.arg("--disable-shared");
```

配置为静态链接模式。

**Line 237**

```rust
configure.arg("--enable-pic");
```

配置生成位置无关代码。

**Line 240**

```rust
configure.arg("--disable-autodetect");
```

禁用库自动探查。

**Line 243**

```rust
configure.arg("--disable-programs");
```

用不到的程序不进行构建。

**Line 245~251**

```rust
macro_rules! enable {
    ($conf:expr, $feat:expr, $name:expr) => {
        if env::var(concat!("CARGO_FEATURE_", $feat)).is_ok() {
            $conf.arg(concat!("--enable-", $name));
        }
    };
}
```

自定义一个macro函数 `enable` ，它会检查特定feature是否在当次构建被指定以此在它的第一个参数也就是configure命令中加入激活选项。

**Line 261~268**

```rust
switch(&mut configure, "BUILD_LICENSE_GPL", "gpl");

switch(&mut configure, "BUILD_LICENSE_VERSION3", "version3");

switch(&mut configure, "BUILD_LICENSE_NONFREE", "nonfree");
```

开源协议相关配置的激活。

**Line 270**

```rust
let ffmpeg_major_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
```

找到ffmpeg主版本号。

**Line 273~279**

```rust
for lib in LIBRARIES
    .iter()
    .filter(|lib| lib.is_feature)
    .filter(|lib| !(lib.name == "avresample" && ffmpeg_major_version >= 5))
{
    switch(&mut configure, &lib.name.to_uppercase(), lib.name);
}
```

基于feature配置是否激活特定库。

**Line 282~283**

```rust
enable!(configure, "BUILD_LIB_GNUTLS", "gnutls");
enable!(configure, "BUILD_LIB_OPENSSL", "openssl");
```

激活SSL库。

**Line 285~333**

```rust
// configure external filters
enable!(configure, "BUILD_LIB_FONTCONFIG", "fontconfig");
// --snip--
enable!(configure, "BUILD_LIB_VMAF", "libvmaf");

// configure external encoders/decoders
enable!(configure, "BUILD_LIB_AACPLUS", "libaacplus");
// --snip--
enable!(configure, "BUILD_LIB_XVID", "libxvid");
```

激活一些ffmpeg需要的外部库。

**Line 336~337**

```rust
enable!(configure, "BUILD_LIB_DRM", "libdrm");
enable!(configure, "BUILD_NVENC", "nvenc");
```

激活一些其他外部库。

**Line 340~341**

```rust
enable!(configure, "BUILD_LIB_SMBCLIENT", "libsmbclient");
enable!(configure, "BUILD_LIB_SSH", "libssh");
```

激活一些外部协议库。

**Line 344**

```rust
enable!(configure, "BUILD_PIC", "pic");
```

激活位置无关代码生成。

**Line 347~360**

```rust
let output = configure
    .output()
    .unwrap_or_else(|_| panic!("{:?} failed", configure));
if !output.status.success() {
    println!("configure: {}", String::from_utf8_lossy(&output.stdout));

    return Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "configure failed {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    ));
}
```

执行配置，并判断是否成功。

**Line 363~371**

```rust
if !Command::new("make")
    .arg("-j")
    .arg(num_cpus::get().to_string())
    .current_dir(&source())
    .status()?
    .success()
{
    return Err(io::Error::new(io::ErrorKind::Other, "make failed"));
}
```

执行 `make` 并判断是否成功。

**Line 374~381**

```rust
if !Command::new("make")
    .current_dir(&source())
    .arg("install")
    .status()?
    .success()
{
    return Err(io::Error::new(io::ErrorKind::Other, "make install failed"));
}
```

执行 `make install` 并判断是否成功。

**Line 383**

```rust
Ok(())
```

返回

### Line 386~403 msvc平台特定调用函数 `try_vcpkg`

```rust
#[cfg(not(target_env = "msvc"))]
fn try_vcpkg(_statik: bool) -> Option<Vec<PathBuf>> {
    None
}

#[cfg(target_env = "msvc")]
fn try_vcpkg(statik: bool) -> Option<Vec<PathBuf>> {
    if !statik {
        env::set_var("VCPKGRS_DYNAMIC", "1");
    }

    vcpkg::find_package("ffmpeg")
        .map_err(|e| {
            println!("Could not find ffmpeg with vcpkg: {}", e);
        })
        .map(|library| library.include_paths)
        .ok()
}
```

对于MSVC编译器执行的特殊处理，仅当编译时指定 `target_env` 选项为 `msvc` 时有意义。依赖 `vcpkg` 库执行ffmpeg系统库的链接。关于 `vcpkg` ：[vcpkg - crates.io: Rust Package Registry](https://crates.io/crates/vcpkg)

### Line 405~605 feature检查函数 `check_features`

```rust
fn check_features(
    include_paths: Vec<PathBuf>,
    infos: &[(&'static str, Option<&'static str>, &'static str)],
) {
    ...
}
```

`check_features` 接收 `include_paths` 和 `infos` 两个参数。 `include_paths` 代表链接库的地址， `infos` 的每一元素为一三元组，形如： `( /path/to/header, feature_name, var)` 。

**Line 409~443**

```rust
let mut includes_code = String::new();
let mut main_code = String::new();

for &(header, feature, var) in infos {
    if let Some(feature) = feature {
        if env::var(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_err() {
            continue;
        }
    }

    let include = format!("#include <{}>", header);
    if !includes_code.contains(&include) {
        includes_code.push_str(&include);
        includes_code.push('\n');
    }
    includes_code.push_str(&format!(
        r#"
        #ifndef {var}_is_defined
        #ifndef {var}
        #define {var} 0
        #define {var}_is_defined 0
        #else
        #define {var}_is_defined 1
        #endif
        #endif
    "#,
        var = var
    ));

    main_code.push_str(&format!(
        r#"printf("[{var}]%d%d\n", {var}, {var}_is_defined);
        "#,
        var = var
    ));
}
```

根据 `infos` 自动生成当前fewature需要的头文件代码保存到 `includes_code` 中， `main_code` 负责打印包含的定义值。

**Line 445~460**

```rust
let version_check_info = [("avcodec", 56, 60, 0, 108)];
for &(lib, begin_version_major, end_version_major, begin_version_minor, end_version_minor) in
    version_check_info.iter()
{
    for version_major in begin_version_major..end_version_major {
        for version_minor in begin_version_minor..end_version_minor {
            main_code.push_str(&format!(
                r#"printf("[{lib}_version_greater_than_{version_major}_{version_minor}]%d\n", LIB{lib_uppercase}_VERSION_MAJOR > {version_major} || (LIB{lib_uppercase}_VERSION_MAJOR == {version_major} && LIB{lib_uppercase}_VERSION_MINOR > {version_minor}));
                "#, lib = lib,
                lib_uppercase = lib.to_uppercase(),
                version_major = version_major,
                version_minor = version_minor
            ));
        }
    }
}
```

将打印对于 `avcodec` 版本兼容性需求的代码加入 `main_code` 。

**Line 462~479**

```rust
let out_dir = output();

write!(
    File::create(out_dir.join("check.c")).expect("Failed to create file"),
    r#"
        #include <stdio.h>
        {includes_code}

        int main()
        {{
            {main_code}
            return 0;
        }}
       "#,
    includes_code = includes_code,
    main_code = main_code
)
.expect("Write failed");
```

将 `include_code` 和 `main_code` 内容写入到 `check.c` 文件中。

**Line 481~518**

```rust
let executable = out_dir.join(if cfg!(windows) { "check.exe" } else { "check" });
let mut compiler = cc::Build::new()
    .target(&env::var("HOST").unwrap()) // don't cross-compile this
    .get_compiler()
    .to_command();

for dir in include_paths {
    compiler.arg("-I");
    compiler.arg(dir.to_string_lossy().into_owned());
}
if !compiler
    .current_dir(&out_dir)
    .arg("-o")
    .arg(&executable)
    .arg("check.c")
    .status()
    .expect("Command failed")
    .success()
{
    panic!("Compile failed");
}

let check_output = Command::new(out_dir.join(&executable))
    .current_dir(&out_dir)
    .output()
    .expect("Check failed");
if !check_output.status.success() {
    panic!(
        "{} failed: {}\n{}",
        executable.display(),
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );
}

let stdout = str::from_utf8(&check_output.stdout).unwrap();

println!("stdout of {}={}", executable.display(), stdout);
```

编译并执行 `check.c` 。

**Line 520~546**

```rust
for &(_, feature, var) in infos {
    if let Some(feature) = feature {
        if env::var(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_err() {
            continue;
        }
    }

    let var_str = format!("[{var}]", var = var);
    let pos = var_str.len()
        + stdout
            .find(&var_str)
            .unwrap_or_else(|| panic!("Variable '{}' not found in stdout output", var_str));
    if &stdout[pos..pos + 1] == "1" {
        println!(r#"cargo:rustc-cfg=feature="{}""#, var.to_lowercase());
        println!(r#"cargo:{}=true"#, var.to_lowercase());
    }

    // Also find out if defined or not (useful for cases where only the definition of a macro
    // can be used as distinction)
    if &stdout[pos + 1..pos + 2] == "1" {
        println!(
            r#"cargo:rustc-cfg=feature="{}_is_defined""#,
            var.to_lowercase()
        );
        println!(r#"cargo:{}_is_defined=true"#, var.to_lowercase());
    }
}
```

在 `stdout` 中逐个匹配应该出现的var是否出现，对于出现的var通过 `rustc-cfg=feature=` instructions设置相应的Cargo feature及环境变量。

**Line 548~473**

```rust
for &(lib, begin_version_major, end_version_major, begin_version_minor, end_version_minor) in
    version_check_info.iter()
{
    for version_major in begin_version_major..end_version_major {
        for version_minor in begin_version_minor..end_version_minor {
            let search_str = format!(
                "[{lib}_version_greater_than_{version_major}_{version_minor}]",
                version_major = version_major,
                version_minor = version_minor,
                lib = lib
            );
            let pos = stdout
                .find(&search_str)
                .expect("Variable not found in output")
                + search_str.len();

            if &stdout[pos..pos + 1] == "1" {
                println!(
                    r#"cargo:rustc-cfg=feature="{}""#,
                    &search_str[1..(search_str.len() - 1)]
                );
                println!(r#"cargo:{}=true"#, &search_str[1..(search_str.len() - 1)]);
            }
        }
    }
}
```

进行版本限制输出的匹配，将匹配成功的 `search_str` 加入到feature和环境变量中。

**Line 575~604**

```rust
let ffmpeg_lavc_versions = [
    ("ffmpeg_3_0", 57, 24),
    ("ffmpeg_3_1", 57, 48),
    ("ffmpeg_3_2", 57, 64),
    ("ffmpeg_3_3", 57, 89),
    ("ffmpeg_3_1", 57, 107),
    ("ffmpeg_4_0", 58, 18),
    ("ffmpeg_4_1", 58, 35),
    ("ffmpeg_4_2", 58, 54),
    ("ffmpeg_4_3", 58, 91),
    ("ffmpeg_4_4", 58, 100),
    ("ffmpeg_5_0", 59, 18),
];
for &(ffmpeg_version_flag, lavc_version_major, lavc_version_minor) in
    ffmpeg_lavc_versions.iter()
{
    let search_str = format!(
        "[avcodec_version_greater_than_{lavc_version_major}_{lavc_version_minor}]",
        lavc_version_major = lavc_version_major,
        lavc_version_minor = lavc_version_minor - 1
    );
    let pos = stdout
        .find(&search_str)
        .expect("Variable not found in output")
        + search_str.len();
    if &stdout[pos..pos + 1] == "1" {
        println!(r#"cargo:rustc-cfg=feature="{}""#, ffmpeg_version_flag);
        println!(r#"cargo:{}=true"#, ffmpeg_version_flag);
    }
}
```

对ffmpeg版本feature进行匹配，并将成功匹配的结果加入到feature和环境变量中。

### Line 607~615 工具函数 `search_include`

```rust
fn search_include(include_paths: &[PathBuf], header: &str) -> String {
    for dir in include_paths {
        let include = dir.join(header);
        if fs::metadata(&include).is_ok() {
            return include.as_path().to_str().unwrap().to_string();
        }
    }
    format!("/usr/include/{}", header)
}
```

用于查询某一头文件的位置。

### Line 617~624 工具函数 `maybe_search_include`

```rust
fn maybe_search_include(include_paths: &[PathBuf], header: &str) -> Option<String> {
    let path = search_include(include_paths, header);
    if fs::metadata(&path).is_ok() {
        Some(path)
    } else {
        None
    }
}
```

用于包装 `search_include` ，因为头文件可能不存在。

### Line 626~637 工具函数 `link_to_libraries`

```rust
fn link_to_libraries(statik: bool) {
    let ffmpeg_ty = if statik { "static" } else { "dylib" };
    for lib in LIBRARIES {
        let feat_is_enabled = lib.feature_name().and_then(|f| env::var(&f).ok()).is_some();
        if !lib.is_feature || feat_is_enabled {
            println!("cargo:rustc-link-lib={}={}", ffmpeg_ty, lib.name);
        }
    }
    if env::var("CARGO_FEATURE_BUILD_ZLIB").is_ok() && cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=z");
    }
}
```

接受参数指定是生成静态还是动态库；根据各library的feature状态决定是否对它们进行链接。库链接的方式使用 `cargo:rustc-link-lib` instructions实现。关于Cargo build脚本的link instructions：[Build Scripts - The Cargo Book (rust-lang.org)](https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script)

### Line 639~1287 主函数 `main`

**Line 640~641**

```rust
let statik = env::var("CARGO_FEATURE_STATIC").is_ok();
let ffmpeg_major_version: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
```

通过环境变量获取链接库类型 `statik` 和 ffmpeg主版本号 `ffmpeg_major_version` 。

**Line 643~677**

```rust
let include_paths: Vec<PathBuf> = if env::var("CARGO_FEATURE_BUILD").is_ok() {
		...
}
```

获取链接库地址，若指定了 `build` feature：

- line 644~647：通过 `rustc-link-search` instruction，编译时添加库索引文件夹即通过 `search` 函数得到输出目的文件夹下的 `lib`
    
    ```rust
    println!(
        "cargo:rustc-link-search=native={}",
        search().join("lib").to_string_lossy()
    );
    ```
    
- line 648：调用 `link_to_libraries` 函数链接到库
- line 649~653：通过 `metadata` 查询静态库文件 `libavutil.a` 是否存在，若不存在则说明库文件没有被生成，调用 `create_dir_all` 函数创建输出文件夹，调用 `fetch` 函数获取ffmpeg源码，调用 `build` 函数构建生成静态库。
    
    ```rust
    if fs::metadata(&search().join("lib").join("libavutil.a")).is_err() {
        fs::create_dir_all(&output()).expect("failed to create build directory");
        fetch().unwrap();
        build().unwrap();
    }
    ```
    
- line 656~674：根据配置文件找到链接的库列表，然后通过 `rustc-link-lib` instructions 指定链接库。
    
    ```rust
    {
        let config_mak = source().join("ffbuild/config.mak");
        let file = File::open(config_mak).unwrap();
        let reader = BufReader::new(file);
        let extra_libs = reader
            .lines()
            .find(|line| line.as_ref().unwrap().starts_with("EXTRALIBS"))
            .map(|line| line.unwrap())
            .unwrap();
    
        let linker_args = extra_libs.split('=').last().unwrap().split(' ');
        let include_libs = linker_args
            .filter(|v| v.starts_with("-l"))
            .map(|flag| &flag[2..]);
    
        for lib in include_libs {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }
    ```
    
- line 676：返回 `include_paths`
    
    ```rust
    vec![search().join("include")]
    ```
    

**Line 679~686**

```rust
else if let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") {
	  let ffmpeg_dir = PathBuf::from(ffmpeg_dir);
	  println!(
	      "cargo:rustc-link-search=native={}",
	      ffmpeg_dir.join("lib").to_string_lossy()
	  );
	  link_to_libraries(statik);
	  vec![ffmpeg_dir.join("include")]
```

若没有指定编译模式则会首先去找环境变量 `FFMPEG_DIR` 它指定了 `FFMPEG` 的位置，在这一模式下会去链接预编译的库，通过 `rustc-link-search` instruction添加预编译库文件夹到库索引。

**Line 687~705**

```rust
} else if let Some(paths) = try_vcpkg(statik) {
    // vcpkg doesn't detect the "system" dependencies
    if statik {
        if cfg!(feature = "avcodec") || cfg!(feature = "avdevice") {
            println!("cargo:rustc-link-lib=ole32");
        }

        if cfg!(feature = "avformat") {
            println!("cargo:rustc-link-lib=secur32");
            println!("cargo:rustc-link-lib=ws2_32");
        }

        // avutil depdendencies
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=user32");
    }

    paths
}
```

若也没有指定预编译库地址，则通过调用 `try_vcpkg` 函数查询是否指定了MSVC编译模式，若是，除了通过 `vcpkg` 链接库文件之外，还要依据feature通过 `rustc-link-lib` instruction指定链接一些该模式下所必须的其他库。

**Line 707~738**

```rust
else {
    pkg_config::Config::new()
        .statik(statik)
        .probe("libavutil")
        .unwrap();

    let mut libs = vec![
        ("libavformat", "AVFORMAT"),
        ("libavfilter", "AVFILTER"),
        ("libavdevice", "AVDEVICE"),
        ("libswscale", "SWSCALE"),
        ("libswresample", "SWRESAMPLE"),
    ];
    if ffmpeg_major_version < 5 {
        libs.push(("libavresample", "AVRESAMPLE"));
    }

    for (lib_name, env_variable_name) in libs.iter() {
        if env::var(format!("CARGO_FEATURE_{}", env_variable_name)).is_ok() {
            pkg_config::Config::new()
                .statik(statik)
                .probe(lib_name)
                .unwrap();
        }
    }

    pkg_config::Config::new()
        .statik(statik)
        .probe("libavcodec")
        .unwrap()
        .include_paths
};
```

若也没有指定MSVC编译模式，则fallback到默认编译模式下。首先通过 `pkg_config` 探查 `avutil` 库，然后将当前feature允许链接的库一一进行链接，最后再编译 `avcodec` 库。

**Line 740~762**

```rust
if statik && cfg!(target_os = "macos") {
    let frameworks = vec![
        "AppKit",
        "AudioToolbox",
        "AVFoundation",
        "CoreFoundation",
        "CoreGraphics",
        "CoreMedia",
        "CoreServices",
        "CoreVideo",
        "Foundation",
        "OpenCL",
        "OpenGL",
        "QTKit",
        "QuartzCore",
        "Security",
        "VideoDecodeAcceleration",
        "VideoToolbox",
    ];
    for f in frameworks {
        println!("cargo:rustc-link-lib=framework={}", f);
    }
}
```

针对MacOS平台进行一些必要组件库的链接。

**Line 764~1064**

```rust
check_features(
    include_paths.clone(),
    &[
        (...
    ],
);
```

调用 `check_features` 对所得到的链接路径基于特征进行检查。

**Line 1066~1068**

```rust
let clang_includes = include_paths
    .iter()
    .map(|include| format!("-I{}", include.to_string_lossy()));
```

格式化 `include_paths` 为 clang 支持的 `-I ...` 命令行选项字符串格式。

**Line 1073~1068**

```rust
let mut builder = bindgen::Builder::default()
    .clang_args(clang_includes)
    .ctypes_prefix("libc")
    // https://github.com/rust-lang/rust-bindgen/issues/550
    .blocklist_type("max_align_t")
    .blocklist_function("_.*")
    // Blocklist functions with u128 in signature.
    // https://github.com/zmwangx/rust-ffmpeg-sys/issues/1
    // https://github.com/rust-lang/rust-bindgen/issues/1549
    .blocklist_function("acoshl")
    .blocklist_function("acosl")
    .blocklist_function("asinhl")
    // --snip--
    .blocklist_function("ynl")
    .opaque_type("__mingw_ldbl_type_t")
    .rustified_enum("*")
    .prepend_enum_name(false)
    .derive_eq(true)
    .size_t_is_usize(true)
    .parse_callbacks(Box::new(Callbacks));
```

创建bindgen builder。

- 通过 `clang_args(clang_includes)` 用于直接传入clang风格参数执行绑定；
- 通过 `ctypes_prefix("libc")` 让原始类型使用 `libc` 前缀；
- 为防止550issue提的bug，通过 `blocklist_type("max_align_t")` 阻止对 `max_align_t` 类型的绑定；
- 通过 `blocklist_function()` 阻止一些函数的绑定；
- 通过 `opaque_type("__mingw_ldbl_type_t")` 让该类型opaque，因为bindgen无法对其正确处理；
- 通过 `rustified_enum("*")` 让任意enum成为Rust enum；
- 通过 `prepend_enum_name(false)` 不预置enum name到常量或者新类型变量上；
- 通过 `derive_eq(true)` 让 `Eq` trait默认获得；
- 通过 `size_t_is_usize(true)` 让 `size_t` 转换为 `usize` ；
- 通过 `parse_callbacks(Box::new(Callbacks))` 自定义配置解析；

**Line 1072~1269**

```rust
if env::var("CARGO_FEATURE_AVCODEC").is_ok() {
    builder = builder
        .header(search_include(&include_paths, "libavcodec/avcodec.h"))
        .header(search_include(&include_paths, "libavcodec/dv_profile.h"))
        .header(search_include(&include_paths, "libavcodec/avfft.h"))
        .header(search_include(&include_paths, "libavcodec/vorbis_parser.h"));

    if ffmpeg_major_version < 5 {
        builder = builder.header(search_include(&include_paths, "libavcodec/vaapi.h"))
    }
}

if env::var("CARGO_FEATURE_AVDEVICE").is_ok() {
    builder = builder.header(search_include(&include_paths, "libavdevice/avdevice.h"));
}

if env::var("CARGO_FEATURE_AVFILTER").is_ok() {
    builder = builder
        .header(search_include(&include_paths, "libavfilter/buffersink.h"))
        .header(search_include(&include_paths, "libavfilter/buffersrc.h"))
        .header(search_include(&include_paths, "libavfilter/avfilter.h"));
}

if env::var("CARGO_FEATURE_AVFORMAT").is_ok() {
    builder = builder
        .header(search_include(&include_paths, "libavformat/avformat.h"))
        .header(search_include(&include_paths, "libavformat/avio.h"));
}

if env::var("CARGO_FEATURE_AVRESAMPLE").is_ok() {
    builder = builder.header(search_include(&include_paths, "libavresample/avresample.h"));
}

builder = builder
    .header(search_include(&include_paths, "libavutil/adler32.h"))
    .header(search_include(&include_paths, "libavutil/aes.h"))
    // --snip--
    .header(search_include(&include_paths, "libavutil/xtea.h"));

if env::var("CARGO_FEATURE_POSTPROC").is_ok() {
    builder = builder.header(search_include(&include_paths, "libpostproc/postprocess.h"));
}

if env::var("CARGO_FEATURE_SWRESAMPLE").is_ok() {
    builder = builder.header(search_include(&include_paths, "libswresample/swresample.h"));
}

if env::var("CARGO_FEATURE_SWSCALE").is_ok() {
    builder = builder.header(search_include(&include_paths, "libswscale/swscale.h"));
}
```

为当前feature需要链接的各个库的头文件加入到builder的header中。

**Line 1271~1275**

```rust
if let Some(hwcontext_drm_header) =
    maybe_search_include(&include_paths, "libavutil/hwcontext_drm.h")
{
    builder = builder.header(hwcontext_drm_header);
}
```

尝试查找 `libavutil/hwcontext_drm.h` 头文件，若存在则将其加入到builder的header中。这一设计应该是为兼容性考虑。

**Line 1278~1281**

```rust
let bindings = builder
    .generate()
    // Unwrap the Result and panic on failure.
    .expect("Unable to generate bindings");
```

通过builder调用 `generate` 生成bindings。

**Line 1284~1286**

```rust
bindings
    .write_to_file(output().join("bindings.rs"))
    .expect("Couldn't write bindings!");
```

将生成的bindings写入到 `$OUT_DIR/bindings.rs` 中。

## 0x04 Summary

ffmpeg-next 项目 `build.rs` 编写逻辑并不复杂，具体步骤如下：

1. 找到静态库的位置即 `include_paths` ，对于静态库根据链接模式的不同选择会导向多种寻路过程：
    - **静态链接模式**：直接指定链接库索引位置在目标目录下的 `lib` 目录（ `cargo:rustc-link-search`）并指定链接库（ `cargo:rustc-link-lib`），在目标目录下寻找静态库，静态库不存在就在线拉取ffmpeg源码并进行配置编译过程，生成静态库，然后根据生成的配置文件进行额外链接库的链接指定，返回 `include_paths` 即生成库目录下的 `include` 目录地址。
    - **预编译模式**：直接搜索 `FFMPEG_DIR` 环境变量对应的预编译好的FFmpeg目录，将其下的 `lib` 目录加入链接库索引位置列表（ `cargo:rustc-link-search`）并指定链接库（ `cargo:rustc-link-lib`），返回预编译目录的 `include` 目录地址。
    - **MSVC模式**：通过使用 `vcpkg` 进行库链接并指定额外需要的库进行链接（ `cargo:rustc-link-lib`）。
    - **默认模式**：默认认为库均已编译链接完成，通过 `pkg_config` 对需要的库进行搜索再返回搜索到的`include_paths` 。
2. 探测是否是MacOS平台，若是则加入特殊组件（ `cargo:rustc-link-lib=framework=`）。
3. 基于feature和预设定的check列表进行 `check_features` ，这一步目前感觉主要用于打印信息。
4. 创建bindgen的builder，为其 `clang_args` 生成基于 `include_paths` 指示库位置的 `-I ...` 形式字符串，进行一系列绑定配置，屏蔽会引发错误的类型和函数。
5. 基于feature信息，对builder进行ffmpeg各个库需要绑定头文件的加入。
6. 调用 `builder.generate()` 生成bindings。
7. 将bingdings写入到 `bindings.rs` 中。

## 0x05 Discussion

通过对该项目的学习分析，个人觉得对 `build.rs` 的书写已经没什么问题了，同时基本上也对Cargo的编译流程有了很深入的了解，非常推荐大家看完Cargo Book后来学习一下，一个实践项目可以将分散的知识点得到汇聚，这样就不太容易忘了。

当然，本文并没有提到编译后如何调用库中的函数，那个目前感觉相对简单，主要是通过 `extern` 进行声明，再用 `unsafe` 包裹函数调用即可。

分析完了，当然要自己上手写了！
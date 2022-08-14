# 深入学习Rust三周所感


## 第三周学习体会

之前可以说是拖更了一周，但也确实觉得没啥成果，因为大部分精力集中于啃rust-bindgen了。

### Rust语法学习心得

本周对于Rust语法的学习还是以看[Rust bible](https://doc.rust-lang.org/book)为主，终于看完了第18章，还剩下19章高级特性和20章的一个最终项目，下周会更体会。其实看完15章智能指针之后这本Bible的精华感觉就吸收差不多了。16章是并行编程，书里讲得并不复杂，可能实践时候会有问题；17章讲面向对象跟看小说似的，关于OOM等等的语言设计哲学也是老生常谈了，书里主要指Rust吸收了OOM中较好的思想，而不满足的情况通过Rust提供的特性也能进行替代性实现；18章实际是一个关于模式匹配的reference，比较简单。后续的19章也是一个reference。所以其实Rust的后面几章还是看得比较轻松的。

### Rust bindgen学习心得

另外本周的一个重点是对bindgen的学习。[bindgen](https://crates.io/crates/bindgen) 是一个Rust用于使用外部C/C++语言库的FFI绑定生成工具。为了后续的项目开发，需要对该工具的使用进行学习。

我的经验是你直接去看bindgen的文档估计还是一头雾水，所以可以先去看看[Cargo Book](https://doc.rust-lang.org/stable/cargo/)，从目的上分析，外部库的链接过程从属于编译工具执行，那么对于Rust自然离不开它的编译工具，而Cargo正是Rust的包经理，同时它也担负起了程序的编译工作。对于Cargo Book，仅需重点阅读它的第二章即User Guide，然后重点阅读第三章Reference中的Build Scripts部分，它重点介绍了使用bindgen需要编写的 `build.rs` 程序在Rust程序编译中的作用。

我的另一个经验是结合实际工程项目的分析来增进对新工具库的理解是非常有帮助的，为此我详细分析了[ffmpeg-next](https://github.com/zmwangx/rust-ffmpeg)项目是如何使用bindgen的：[决战Rust bindgen](https://zhuanlan.zhihu.com/p/548743006)。通过实际工程的分析，能够对 `bindgen` 的具体过程有更好了解，同时在分析过程中遇到问题及时查看Cargo Book中的reference，这样对Rust程序工程的理解也更进了一步。


## 对Rust及bindgen的思考

其实 `bindgen` 方式并不是唯一的解决方案，但它是一种进行项目拓展时可能的高效益方式，其实我们完全可以不去调用外部库，从0开始造轮子，Rust这样的底层语言也支持这样去做，但它的成本太高了，而且很可能不利于生态的建立。

在上个月举行的CPP North C++大会上，Google介绍了它们正打造的用于兼容C++代码进行拓展的新语言[Carbon](https://github.com/carbon-language/carbon-lang)。其中Google就提到了Rust对C++的兼容性问题。关于Carbon的讨论已经有了许多，我认为还挺有意思的，大家可以关注关注。但我对Rust对C++兼容性不佳这个问题的出发点抱有疑问，因为Rust本就不是为了兼容C++的，而是取代它，但如果大家都不愿意接纳Rust，仍固守着C++，那只能说Rust做得还不够好，同时它做得不好的地方也正是Rust所需要努力的方向。

## 关于本系列的后话

为了激励我自己积极坚持学习Rust，我会在以后的每次更新后提前预告下一期的内容。

下一期的内容预告：
- Rust bible完结体会
- 浅析为什么Rust能够保障安全性以及它有什么不足

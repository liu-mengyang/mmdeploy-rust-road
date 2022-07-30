# 深入学习Rust两周所感

## 第二周学习总结

还是以阅读和实操为主，主要学习[Rust bible](https://doc.rust-lang.org/book)和[Rust by example](https://doc.rust-lang.org/stable/rust-by-example)。

额外地，将[Tokio的tutorial](https://tokio.rs/tokio/tutorial)作为了一个实战项目进行练习，能将阅读过程中看到的知识点串接起来，效果还是很不错的。

### 第二周学习方法与进度

基本延续了第一周的方法，阅读和学习过程中直接记录英文笔记，对于不理解的代码段进行手敲实现。目前Rust by example除了22节中对行内汇编即关于 `asm!` macro的使用暂时跳过后，其他小节均已学习完毕。Rust bible看完了16章。

另外，完成了Tokio tutorial中的简易版，这是一项非常好的练习！

### 第二周学习效果

对Rust语法的熟悉程度越来越高了，特别在学习智能指针即bible第15章后，整体对Rust的认知水平感觉提升了一个级别。目前基本能阅读大部分的Rust代码，并且可以书写一些简易功能模块。

## 学习体会

似乎来到了学习过程中的第一个信心峰值，可能就是看完新手教程后的那种不真实的膨胀感hhhh。

**关于Rust bible和Rust by example：**

Rust by example是一个非常好的Rust快速学习教程，如果你是赶项目赶工程，可以花一周时间就看Rust by example，那样基于别人的代码进行读写修改应该没啥问题；但如果你是一个像我一样期望对Rust能有更深入理解，后期想自己动手完整开发一个项目的开发者来说，也许Rust bible还是应该啃一啃的，它会让你知道更多Rust内部的机理以及Rust语言中各机制设计的意义及各自的取舍。

## 对Rust的思考

实际上上一周思考的疑惑还没有完全解开，进一步讲在学习更多Rust关于智能指针和并行方面内容后，关于Rust的安全性保障和性能保障这两方面的疑惑又加深了。Rust在尽可能为安全保驾护航，它也做得非常好，但确实有时为了保障安全需要牺牲性能，这也许是不可避免地，Rust在尽可能做到最好。

## 新的规划与后话

我写得这些仅仅是感悟，也许有人会觉得不是干货价值不高，但干货很干，写得干读得也干，我的想法是参考别人的一手经验以及推荐的一手学习资料进行学习是最佳的，而不是参考别人的二手学习资料学习。

正如上一话所说，我的Rust学习阶段性目标是为mmDeploy库增添Rust API接口，为此，我开辟了一个仓库[mmdeploy-rust-road](https://github.com/liu-mengyang/mmdeploy-rust-road)，这个仓库会详细记录我的学习和开发过程，目前还处于学习阶段。开辟代码仓库以希望能给同样想学习Rust的开发者们或者仅仅是阅读本文收获乐趣的朋友一种更加感同身受的体验。

后面会学习关于[rust-bindgen](https://docs.rs/bindgen/latest/bindgen/)的知识，我也会将学习心得继续进行分享！
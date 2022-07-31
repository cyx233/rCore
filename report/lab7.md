# Chapter 7
## 实验内容

- 合并前面的代码
- File trait 中加入get_stat函数，返回Stat结构体
- 在OSInode中实现get_stat
- 在Inode中增加linkat与unlinkat函数，只有ROOT_INODE调用
- 在Inode中加入记录每个id对应的link数量的Vec成员，只有ROOT_INODE使用

## 问答问题

1. 大部分代码能够复用，只需要修改open_file的逻辑。在打开文件时，需要解析路径，
如果路径中存在dir，则将此dir做为新的搜索起点，再次调用open_file，最终
递归返回搜索得到的fd即可。操作系统层面所有对文件的操作都基于open_file函数。
根目录仍然保存硬链接的数量记录，其他目录节点不需要。

在sys_open中，已经留有_dir_fd的参数，实际的实现可以省去解析的步骤，操作系统只需要
提供在指定dir中open指定文件的接口，由用户库完成path的解析，并多次调用sys_open直至最终获得文件。

2. 测试系统：5.4.91-microsoft-standard-WSL2:Ubuntu 20.04 LTS

	实现方式：希望实现目录结构 father/son/father_is_son，其中father_is_son 是 father 目录的硬链接

	实验结果：
	```
	(base) 
	# cyx @ chen-pc in ~/School/father/son [16:37:42] 
	$ ln ../../father father_is_son
	ln: ../../father: hard link not allowed for directory
	```
	Linus系统禁止对目录使用硬链接，从根本上保证文件系统有向无环图的特性。不过允许软链接 
	```
	(base) 
	# cyx @ chen-pc in ~/School/father/son [16:37:58] C:1
	$ ln -s ../../father father_is_son
	(base) 
	# cyx @ chen-pc in ~/School/father/son [17:05:15] 
	$ l                               
	total 8.0K
	drwxr-xr-x 2 cyx cyx 4.0K May  3 17:05 .
	drwxr-xr-x 3 cyx cyx 4.0K May  3 16:37 ..
	lrwxrwxrwx 1 cyx cyx   12 May  3 17:05 father_is_son -> ../../father
	```
	软连接是独立的文件，有独立的inode ID，只是特殊的文件内容，此结构与无环图不矛盾。
	但是另一种结构可以形成软链接的环路，如下
	(base)
	```
	# cyx @ chen-pc in ~/School [20:58:50] 
	$ ln -s 1 2                       
	(base) 
	# cyx @ chen-pc in ~/School [20:58:59] 
	$ ln -s 2 1
	(base) 
	# cyx @ chen-pc in ~/School [20:59:02] 
	$ cd 1     
	cd: too many levels of symbolic links: 1
	```
	此时类似cd会被terminal中止

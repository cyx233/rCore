# Chapter 6
## 实验内容

- 合并前面的代码
- 新增了MailBox类，用于邮箱的思想
- 在 `task::manager` 增加通过 `pid` 发送邮件的功能
- 在 `syscall::fs` 中实现 `sys_mailread` 和 `sys_mailwrite` 函数。在 `syscall::mod` 中增加相应分支

## 问答题

1. shell中 `|` 操作符利用了pipe。shell创建一个pipe，`|` 前的进程的stdout被shell重定向到pipe的写文件上， `|` 后的进程的stdin重定向到pipe 的读文件上。
   

2. 答：
   1. 可能会出现多个生产者同时向一个消费者发邮件，产生冲突，使得消费者收到的邮件数量不对，或者内容错误。

		单核也会发生问题。如果在内核态允许时钟中断(实验中不允许)，那么可能A进程在执行发邮件到B进程时的功能时被打断，切换到C进程，C进程如果再次发邮件到A进程，就会导致两次写入同一个邮件 buffer，使得A进程 收到的邮件出错。


   2. 伪码实现
	临界区变量
	```
	AR = 0;   // # of active readers
	AW = 0;   // # of active writers
	WR = 0;   // # of waiting readers
	WW = 0;   // # of waiting writers
	Lock lock;
	Condition okToRead;
	Condition okToWrite;
	```
	读者 
	```
	Private Database::StartRead() {
		lock.Acquire();
		while ((AW+WW) > 0) {
			WR++;
			okToRead.wait(&lock);
			WR--;
		}
		AR++;
		lock.Release();
	}
	Private Database::DoneRead() {
		lock.Acquire();
		AR--;
		if (AR ==0 && WW > 0) {
			okToWrite.signal();
		}
		lock.Release();
	}
	Public Database::Read() {
		//Wait until no writers;
		StartRead(); 
		read database;
		//check out – wake up waiting writers; 
		DoneRead(); 
	}

	```

   	写者
    ```
	Private Database::StartWrite() {
		lock.Acquire();
		while ((AW+AR) > 0)  {
			WW++;
			okToWrite.wait(&lock);
			WW--;
		}
		AW++;
		lock.Release();
	}
	Private Database::DoneWrite() {
		lock.Acquire();
		AW--;
		if (WW > 0) {
			okToWrite.signal();
		}
		else if (WR > 0) {
			okToRead.broadcast();
		}
		lock.Release();
	}
	Public Database::Write() {
		//Wait until no readers/writers;
		StartWrite(); 
		write database;
		//check out-wake up waiting readers/writers; 
		DoneWrite(); 
	}
	```
	3. (1)以读写一个报文为原子操作，使用管程实现互斥。这样子多线程就没有影响了。(2)由于是顺序访问FIFO，所以可以使用队列的数据结构，简化实现，也可以实现以节点为粒度的互斥锁，提高性能。

      

   
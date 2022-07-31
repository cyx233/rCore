# Chapter 3
## 实验内容
1. 实现了抢占式调度系统
2. 实现了heap进行优先级管理
3. 实现对应用的计时，超时杀死应用
## 实验结果
见CI-CD
## 简答作业
1. 在示例实现中，使用了轮转管理。在自己的实现中，任务信息由数组管理，使用heap管理下标，进行调度。

   进程切换有两种情况：
      1. 发生时钟中断，OS响应时钟中断并切换进程。
      2. 进程主动调用sys_yield()，进行切换。

	选择下一个运行的进程的策略：

	在示例实现中，第i个进程suspend后，会以 i -> max_num -> 0 -> i 的顺序遍历进程池，选择第一个Ready的进程。

	在自己的实现中，每个进程有一个优先级priority:usize，一个进度stride:usize。选定一个足够大的数BIG_STRIDE。每次运行时，stride增加BIG_STRIDE/priority，调度策略即为选择当前stride最小的进程。

	加入新进程时，在示例实现中，直接加到队尾。在自己的实现中，stride初始化为0，加入task_list，以new_id表示其下标。由于实现了heap接口进行调度管理，只需要调用push_app(new_id)即可。

2. 答：

	2.1 从当前来看并没有实质的不同，都是在固定长度的列表中进行进程管理，进程的id不重复使用。
	如果有新进程的产生，VecDeque会将新进程放在队尾，保持先产生先运行。而列表需要把新进程放在空位，不保序。

	2.2 假设5个进程

 | 时间点   | 0        | 1      | 2               | 3              | 4   | 5   |
 | -------- | -------- | ------ | --------------- | -------------- | --- | --- |
 | 运行进程 |          | p1     | p2              | p3             | p5  | p4  |
 | 事件     | p1-3产生 | p1结束 | p4产生(1是空位) | p5产生（队尾） |     |     |

	产生顺序：p1,p2,p3,p4,p5
	执行顺序：p1,p2,p3,p5,p4

	在我实现的调度算法下，理想执行顺序是p1,p2,p3,p4,p5。由于新产生的进程push到heap底部，上浮时不会打乱顺序。但是如果连续插入填满了右子树，而后插入到了左子树，那么由于下沉时优先左子树，会让更晚插入的在左子树的进程先被上浮，无法得到理想的执行顺序。

3. 不是，因为溢出了。p1.stride=255,p2.stride=250+10-256=4,继续由p2执行。

	可通过归纳法证明:

	不妨设第i步pk.stride最小，表示为pk(i).stride=STRIDE_MIN。
	
	假设第i步 STRIDE_MAX-pk(i).stride=delta <= BIG_STRIDE/2。

	由于pk.prioriy>=2，所以pk.pass<=BIG_STRIDE/2。在pk运行之后，有pk(i).stride <= pk(i+1).stride = pk(i).stride+pk.pass <= pk(i).stride+BIG_STRIDE/2。

	1. pk(i+1).stride仍是最小的，STRIDE_MAX-pk(i+1).stride <= STIRDE_MAX-pk(i).stride = delta < BIG_STRIDE/2
	2. pk(i+1).stride不是最小的，但也不是最大的，不妨设此时pm(i+1).stride最小，显然有pm(i+1).stride = pm(i).stride >= pk(i).stride，STRIDE_MAX - pm(i+1).stride <= STRIDE_MAX - pk(i).stride = delta <= BIG_STRIDE/2
	3. pk(i+1).stride是最大的，不妨设此时pm(i+1).stride最小，显然有pm(i+1).stride = pm(i).stride >= pk(i).stride，pk(i+1).stride-pm(i+1).stride <= (pk(i).stride+BIG_STRIDE/2)-pk(i).stride = BIG_STRIDE/2

	综上，所以情况下都有第i+1步 STRIDE_MAX-STRIDE_MIN <= BIG_STRIDE/2。

	由第0步STRIDE_MAX-STRIDE_MIN=0。由归纳原理，得证任意步骤中 STRIDE_MAX-STRIDE_MIN <= BIG_STRIDE/2。

4. 
```rust
impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.0 < other.0{
			let delta = other.0 - self.0;
			if(delta<=BIG_STRIDE/2){
				return Some(Ordering::Less);
			}
			else{
				return Some(Ordering::Greater);
			}
		} else {
			let delta = self.0 - other.0;
			if(delta<=BIG_STRIDE/2){
				return Some(Ordering::Greater);
			}
			else{
				return Some(Ordering::Less);
			}
		}
    }
}
```

wait is a control flow statement that suspends execution of a thread pending receipt of a message of a given type.
It can listen for multiple message types using the on statement.
When a wait statement is executed, execution stops on the thread until one of the on conditions is met.

```gs
wait()
{
    on "hello", "world" :
    {
        // code here will execute for any hello,world message and then continue to wait for other messages.
        continue;
    }
    on "hi", "there" :
    {
        // code here will execute for any hi,there message and then exit the loop.
        break;
    }
    on "hey", "" :
    {
        // code here will execute for any message with a major type of
        // "hey" this will break by default, terminating the wait() loop.
    }
}
```
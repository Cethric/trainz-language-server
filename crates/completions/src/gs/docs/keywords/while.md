- A `while(condition)` is a loop control method that will continue as long as its condition evaluates to `true`.
- Note that if the condition is initially `false` the loop will not run at all

```gs
int i = 5;
while (i > 0)
{
    Interface.Print("i is greater than zero (" + i + ")");
    --i;
}
```
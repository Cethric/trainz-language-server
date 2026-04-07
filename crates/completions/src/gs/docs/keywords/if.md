- if and else are keywords for the construction of if / then / else constructs used to control program flow
- Code following an if(condition) statement is executed only when the condition evaluates to true.
- In any other case execution branches to any immediately following else statement.
- else and else if clauses are optional.

```gs
int i = Math.Rand(0, 3);

if (i == 0)
{
    Interface.Print("i is 0");
}
else if (i == 1)
{
    Interface.Print("i is 1");
}
else
{
    Interface.Print("i is 2");
}
```

- Exits from current code block (for, wait, switch, while)
- Execution will continue with the first statement after the loop.

```gs
// This loop will complete when n is greater than 5 or when n == list.size(), whichever occurs first
for (n = 0; n < list.size(); ++n)
{
    if (n > 5)
        break;
}
```

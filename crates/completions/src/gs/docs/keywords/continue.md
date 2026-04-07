- Jumps to the next iteration of a for, while or wait loop For example:

```gs
int i;
for(i = 0; i < 10; ++i)
{
    if (i == 5)
        continue;
    Interface.Print(i);
}
```

- The above code will print 0 to 4, skip 5 and then print 6 to 9.
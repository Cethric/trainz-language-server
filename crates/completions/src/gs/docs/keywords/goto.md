- The goto statement will branch to a label statement. For example:

```gs
int i = 5;

label_A:
Interface.Print("hello");
if (i--)
    goto label_A;
```

- The above code will print hello 5 times. (zero evaluates as boolean false)
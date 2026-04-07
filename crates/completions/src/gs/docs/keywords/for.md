- for is a control flow loop statement.
- A for loop has four parts and takes the form: for(A; B; C) D where:
    - A initialises a counter variable, and is executed before the looping begins
    - B is the test condition, looping will continue while B is true
    - C is the loop expression, and will be executed at the end of every loop
    - D is the code body statement, usually a compound statement, and is executed on every loop.

```gs
Junction[] junctions = GetJunctionList();
int i;
for (i = 0; i < junctions.size(); ++i)
{
string name = junctions[i].GetLocalisedName();
Interface.Print("Junction " + i + " is named: " + name);
}
```

- Depending on the values of Parts A and B the loop may not execute at all.
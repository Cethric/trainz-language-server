- Individual case statements are labels that the code will jump to dependent on the value of the integer expression
  following the keyword.
- The case statement is used in the switch control flow statement to declare an integer option and the action to be
  taken when execution is transferred to this label.
- This code will print the days of the week from Sunday to Saturday:

```gs
int i;
string dayOfWeek;
for(i = 0; i < 6; ++i)
{
    switch(i)
    {
        case 0:
            dayOfWeek = "Sunday";
            break;
        case 1:
            dayOfWeek = "Monday";
            break;
        case 2:
            dayOfWeek = "Tuesday";
            break;
        case 3:
            dayOfWeek = "Wednesday";
            break;
        case 4:
            dayOfWeek = "Thursday";
            break;
        case 5:
            dayOfWeek = "Friday";
            break;
        case 6:
            dayOfWeek = "Saturday";
            break;
        default:
            dayOfWeek = "ERROR";
            break;
    }
    Interface.Print(dayOfWeek);
}
```

- switch transfers execution to the statement immediately following the appropriate case label.
- If there is no match execution continues after the default label.
- Note that the list of statements following the case selector should normally terminate with break; (which exits from
  the switch statement) or return (which exits from the entire method) to prevent the code from running on into the next
  section.
- A class or method specifier that prevents users from overriding the declaration

```gs
final class Info
{
    string name;
    string address;
}
```

- Given the above the declaration the following declaration would not be possible

```gs
class DetailedInfo isclass Info
{
    string telephone;
}
```
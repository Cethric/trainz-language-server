- static is a class specifier denoting that only one instance of the class may exist.
- Static classes cannot be created using the new operator.
- To access a static class refer to its members or methods using the class name as follows:

```gs
static class Owner
{
    public string forename;              // Public member variable
    public string surname;               // Public member variable
    
    // Public function
    public string GetName(void)
    {
        return forename + " " + surname;
    }
    
};
    
class Postbox isclass MapObject
{
    public void Init(void)
    {
        if (!Owner.forename)
            Owner.forename = "Fred";
        if (!Owner.surname)
            Owner.surname = "Bloggs";
        Interface.Print(Owner.GetName());
    }
}
```

- Since only one instance of owner may exist it will be created along with the first instance of the postbox obect
- Any future postboxes will find the members of the owner class already initialised and ready for use.
- A thread is a method which runs independently of the main script code.
- threads can be started and synchronised by standard methods or other threads but are allowed to wait for events or to
  carry out tasks requiring periods of inactivity without interrupting the main program flow.
- Threaded methods may only be declared in classes extending the GameObject class or its children.
- Many threads (up to 64) may be running on the same game object instance at a time.

```gs
class Tutorial isclass Buildable
{
  bool bThreadRunning = false;
  bool doorsOpen = false;
  
  public void Init(void)
  {
    inherited();
    AddHandler(me, "Object", "", "ObjectHandler");
  }
  
  thread void RingTheBell(void)
  {
    // Make sure we don't run multiple 'RingTheBell' threads at once.
    if (bThreadRunning)
      return;
    bThreadRunning = true;
    
    while (doorsOpen)
    {
      // Play sound file
      World.Play2DSound(GetAsset(),"bell.wav");
      
      // Wait for sound file to finish
      Sleep(0.35);
    }
    
    bThreadRunning = false;
  }
  
  void ObjectHandler(Message msg)
  {
    if (msg.minor == "Enter")
    {
      doorsOpen = true;
      RingTheBell();
    }
    else
    {
      doorsOpen = false;
    }
  }

};
```

- In the above the thread RingTheBell is started on an Object,Enter message and will terminate on any other Object
  message.

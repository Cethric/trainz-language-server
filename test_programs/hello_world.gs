include "Bar.gs"
include "baz.gs"

class Foo isclass Bar, Baz {
    public int x = 1 + 2 * 3;
    static string name = "Blake";

    void run(int a, int b) {
        if (a < b) {
            x = a;
        } else {
            x = b;
        }
    }

    native int compute(float v);

    pub example() {
        inherited()
    }
};
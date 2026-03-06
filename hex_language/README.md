# Example usage

```lua

local function vec2(T)
    return struct {
        {x, T},
        {y, T}
    }
end

local x = place(vec2(bi32), 0x00)
assert(x:range().max == 0x08)
-- __index forwards to :value
assert(x.x:value() == 1234)
```

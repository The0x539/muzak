# indices ? fps tuning sampleRate sampleCount
GetFrameIndices ← (
  ⊙⊙÷    # Duration
  ∩⌟(⌈×) # Desired vs actual frame count
  # Actual frames, nearest-neighbor interpolated to desired frame positions
  ⁅×⊙-₁÷⟜⇡
)

Encode ← ⇌°⋯ ⍜(⊣°↯)(˜⊂¯8⌈÷8)

PlotCurve ← ↧⊓⌞⊞≤⊞≥ ⊓⇡(⊃↧↥⊸(⊂⊸⊢↘¯1))

For! ← ◌⍥(+1◡^0)⊙0

Render ← (
  ⬚0↙800↘⊡
  ⁅×99
  PlotCurve 100
  ⬚0↙100
)

# ? fps tuning sampleRate audio frameSink
Main ← (
  ⁅×⊃(⋅÷|GetFrameIndices⊸⊙⊙⊙⧻)
  ⊙(⌝↘800)
  ⊙(÷⊸/↥ -⊸/↧)

  ◌◌For!(&w Encode Render) ⊸⧻
  &cl
)

≡wait≡spawnMain

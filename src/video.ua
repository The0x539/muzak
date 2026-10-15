# indices ? fps tuning sampleRate sampleCount
GetFrameIndices ← (
  ⊙⊙÷    # Duration
  ∩⌟(⌈×) # Desired vs actual frame count
  # Actual frames, nearest-neighbor interpolated to desired frame positions
  ⁅×⊙-₁÷⟜⇡
)

Pad    ← ⬚0↙ ⊂100 ⍜÷₈⌈⊣⊸△
Encode ← °⋯ ↯∞_¯8 ⇌ Pad

# frame ? height samples
PlotCurve ← ↧⊓⌞⊞≤⊞≥ ⊓⇡(⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

# frame ? position audio
Render ← PlotCurve 100 ⬚50↙800↘

Remap ← ⁅×99 ÷⊸/↥ -⊸/↧ ⌝↘800

# ? fps tuning sampleRate audio frameSink
Main ← (
  ⁅×⊃(⋅÷|GetFrameIndices⊸⊙⊙⊙⧻)
  ⊙Remap
  &cl⊸≡⌟₂(&w Encode Render)
)

≡wait≡spawnMain

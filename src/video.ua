# indices ? fps audio
GetFrameIndices ← (
  ⊙⧻        # Sample count
  ⊙⊸(÷&asr) # Duration
  ⌈×        # Frame count
  ÷⟜⇡       # 0..1
  ⁅×        # 0..samples
)

# Choose the argument with the smaller absolute value
MinMag ← ⨬∘⋅∘ <◡∩⌵

# Find "features" in the audio, currently defined as positive zero-crossings
Features ← ˜⊂0 >⊃↘₋₁↘₁ ±

# ↙, but accepts a length larger than the array
🪚 ← ↙↧⊙⊸⧻

# Find the distance to the nearest `feature` at/after `index`, up to `radius`.
# offset ? index features radius
🔎 ← ⬚∞⊢⊚ ˜🪚↘
# Find the distance to the nearest `feature` at/before `index`, up to `radius`.
# offset ? index features radius
🔍 ← ¯⬚∞⊣⊚ ˜↘ ⊙(↥₀˜-)⟜⧻ 🪚+₁

InfToZero ← ⨬∘⋅0 =∞⊸⌵

# indices ? indices features radius
🔜 ← + InfToZero MinMag ⤚⊃≡🔍≡🔎 ⊙¤

# indices ? indices audio size
SnapFrameIndices ← ⍜+🔜 ⊃(⁅÷₂⋅⋅⊣|∘|⋅Features|/↥⧈-)

# Add zeroes to a frame to ensure its dimensions are height×8ℤ
# frame ? frame size
Pad ← ⬚0↙ ⊂ ⤚₂⋅⋅⊢ ⍜÷₈⌈ ⊸⧻₁
# frame ? frame size
Encode ← °⋯ ↯∞_¯8 ⇌ Pad

# frame ? samples size
PlotCurve ← ↧⊓⌞⊞≤⊞≥ ˜⊓(⇡⊢|⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

# samples ? position audio size
🍕 ← ⬚÷₂↙ ⤚⋅°⊟ ↘

# frame ? position audio size
Render ← PlotCurve⊸🍕

# audio ? audio size
Remap ← ⁅× ⊙(-1⊢) ÷⊸/↥ -⊸/↧

# ? fps audio size frameSink
Main ← (
  ⊸GetFrameIndices
  ⊸₂SnapFrameIndices
  ⊙⊸Remap
  ≡⌟₃(&w Encode ⊸Render)
)

⊙◌ # TODO: make dynamic strobe optional

wait≡spawn◇Main

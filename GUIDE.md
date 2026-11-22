# MultiPass – Quick guide

MultiPass merges several NC programs from Fusion 360 Personal into **one** program with tool
changes. Everything works with the mouse.

## In 5 steps

1. **Add files:** drag NC files into the window or click "Choose files".
   Several files are sorted by number (1_…, 2_…, 10_…).
2. **Check the order:** click a file, then "Move up" / "Move down" on the right. Or drag the ⋮⋮ handle.
3. **Check tool numbers:** the T number of each file is shown on the right. Change it with **−** / **+**
   or click the number (number pad 1–99). Known cutters from the **inventory** get their number automatically.
4. **Optional:** turn on "Air moves as rapids" and/or "Spindle fan off during tool changes" (see below).
5. Click **"Merge and save"** and confirm the location (suggestion: `MERGED.cnc` in the folder of
   the first file). **"Open folder"** shows the finished file.

## Air moves as rapids

Fusion Personal writes all rapids as slow feed moves. MultiPass can turn moves **above the stock**
back into rapids. This often saves more than an hour.

- Only moves **entirely above the safe height**, or pure retracts upwards, are converted.
- **Caution:** the program does not know about clamps that are higher than the safe height.
  Set the safe height with − / + (0.5–20 mm).
- Only possible if the **Z origin is set to "Stock Top"** in Fusion.
  Otherwise MultiPass turns the option off for this run and tells you.

## Spindle fan off during tool changes

With this option MultiPass writes `M812` (spindle fan off) before every tool change and
`M811 S100` (fan back on at full power) right after it. The machine then no longer blows dust
around while you swap the cutter. The toolpaths stay unchanged.

## Inventory

Click **"Inventory"** at the top. It lists cutters with a fixed number, e.g. T13 = Spiral O Metal 3.175×12.
To remember a new cutter: select the file, set the number, click **"Save to inventory"**.

## Run time and preview

- The estimated run time is shown at the bottom, with and without rapids. Set the machine's rapid
  speed under "Options" for a better estimate.
- **"Toolpath preview"** tab: top view of all toolpaths, coloured per tool, rapids dashed.
  "Enlarge" shows it full screen.
- **"Simulation"** switch: shows the stock from above as it looks after milling.
  Deeper areas are darker, through cuts are transparent.
  - **Play** lets the tool follow the toolpaths and removes the material as it goes.
  - Clicking or dragging the **timeline** jumps to any point. The coloured sections are the parts
    (one colour per tool).
  - Speed 10× to 2000×. "Pause at tool change" stops where the machine asks for the next cutter.
  - All buttons are available under "Enlarge". The simulation is a simplified top view:
    clamps and holders are not checked.

## On the Makera Z1

- Makera Studio asks **"Please change to Tool n"** at every change. Insert the matching cutter and
  click **Confirm**. The tool is then measured automatically.
- **Probe Auto-Z only on untouched stock**, not in an area that has already been milled.

## If something goes wrong

| Message | What to do |
|---|---|
| "no tool change found" | The file is not a Fusion/Carvera program. Post it again with `Carvera.cps`. |
| "no program end found" | The file is incomplete. Post it again in Fusion. |
| "different units" | Output all files in mm. |
| Warning "Z origin" / "X/Y offsets" | Check that all files come from the same setup. |

# r3bl_rs_utils documentation workflow

1. This folder contains diagrams that are used in the README.md and lib.rs files. These diagrams are
   created in Figma and exported to SVG. The SVG files are then embedded in the README.md and lib.rs
   files and stored in this folder.
2. The MD files in this folder are not meant to be used directly by 3rd party developers. They are
   more of a staging ground for content (design docs, architecture diagrams, etc) as a feature or
   component is being built. Once these are stable, they should be copied to the README.md and
   lib.rs (which is where 3rd party developers will see them).

# Information on managing docs, videos, and images

## figma.com

New diagrams are best created in Figma and then exported to SVG. Draw.io is difficult to use.

## Videos

You can use [Kooha](https://flathub.org/apps/details/io.github.seadve.Kooha) on Linux to record a
video of the [Black Box](https://flathub.org/apps/details/com.raggesilver.BlackBox) terminal app.
Change the default settings:

1. Capture 30 fps.
2. Do not capture the mouse.
3. Save as MP4.
4. Make sure that the video is under 2 min (10M is the limit for github.com).

Once captured you can upload to the following sites:

- Github.com (r3bl_rs_utils repo)

  - Edit an the main [README.md](https://github.com/r3bl-org/r3bl_rs_utils#readme) file and drag and
    drop the MP4 file from your desktop to the editor. This will upload the video to github.com and
    generate a URL like this:
    <https://user-images.githubusercontent.com/2966499/206881196-37cf1220-8c1b-460e-a2cb-7e06d22d6a02.mp4>.
    Make sure to commit the file.
  - More info on how to upload video: <https://stackoverflow.com/a/68269430/2085356>

- Reddit.com (r/rust)

  - Create a new video only post, upload the .MP4 file & add description in a comment below it.
  - Don't include "xbox" in any substring (eg: flexbox) of any text that is typed in the post title.

## README and lib.rs updates

After doing all the steps above, it is necessary to update all the `README.md` and `lib.rs` files w/
the latest docs and links to SVG, MP4, etc.

1. root folder of the repo:

   - `README.md` - the links to SVG, MP4 files are relative to the source file.

2. in `tui` sub-folder, the following files have the same documentation content:

   - `README.md` - the links to SVG, MP4 files are relative to the source file.
   - `src/lib.rs` - the links to SVG, MP4 files are direct to githubusercontent.com. For eg:
     [memory-architecture.drawio.svg](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg).

## draw.io (deprecated)

1. Create a new diagram

   - Create a new diagram using draw.io
   - When you save it, make sure to use File -> Export as -> SVG... Make sure to use these settings:
     - ✅ Transparent Background
     - ✅ Dark
     - ✅ Shadow
     - ✅ Include a copy of my diagram
     - Then Export
   - Our convention is to use ".drawio.svg" as the file extension.

2. Update an existing diagram

   - Open the existing diagram (with .drawio.svg extension) in draw.io
   - Make your changes
   - Follow the same File -> Export as -> SVG... as shown above when it is time to save the file

3. Viewing diagrams

   - By following the above steps, the SVG file is really optimized to be viewed w/ a dark
     background (viewing on github.com or on developerlife.com). And if you try to view it w/ a
     light background, it either won't look good or will be illegible.
     - It is easiest to view the diagram on github.com.
     - You can open the SVG file in a browser, then go to developer options, and apply a CSS
       property of background color of black.

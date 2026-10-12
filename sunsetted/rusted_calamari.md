# APPLICATION DESIGN AND DEVELOPMENT REQUEST

> AppName: Rusted Calamari
> AppDescription: "A Retro Computing inspired Octobrachial Tree Manager​ to manage and display your files in windows."
> Syntax: Rust
> Purpose: Fast and manual image classifier modeled after the Octobrachial Postal Alien​  from Men in Black​
> BEAUTIFULLY RETRO DESIGNED DIRECTORY TREE BROWSER WINDOW, NO TROPES, NO BOILERPLATE. THIRD PARTY ASSETS ARE ACCEPTABLE.
> AT ITS CORE. THIS IS AN IMAGE ORGANIZATIONAL TOOL CREATED IN A NEW AND NOVEL OUTPUT DISPLAY AND OPERATIONAL FORMATS THAT WORK WITH EACHOTHER. ONE DOES NOT NEED TO RENDER THE OTHER, THEY ARE EACHOTHER.
> **ABSOLUTELY NO DASHBOARD STYLE LAYOUTS OR UI'S INVOLVED IN THIS**
> **NO BOILERPLATE**

**WHEN CODE BECOMES DEEPY NESTED, AND THEIR IS SIMILAR BRANCHES, IT OFTEN TURNS INTO WHAT VISUALLY LOOKS LIKE AN OCTOPUS**, THAT IS THE NATURAL WAY OF THINGS, THE NATURAL OPERATION AND ORDER OF ORGANIZATIONS VISUAL OUTPUT;
I WANT TO LEAN INTO THIS DOCTRINE AND DEVELOP INTO IT.

## DESIGN

### UI_DESIGN_THEMES (UID_THEME)

FONT SETTINGS TO FINE TUNE EXPERIENCE

> UID_THEME: "Retro Aesthetic Amplified in Modern Tooling (RAAMT)"

RETRO DESIGN USING MODERN METHODS, TOOLS(THIRD PARTY LIBRARIES AND TOOLING IS APPROVED DURING DEVELOPMENT):

- HOVER_MENU:
  - should only fade in on hover,
  - Nothing about the UI should be visible untill hovered, including the settings menu.
- font options: "PRE INSTALLED FONTS THAT ARE EASILY CHANGABLE FOR THE MENUS, UI, AND PREVIEW WINDOW TREES."
  - [IBM Model 13x Alt1](https://int10h.org/oldschool-pc-fonts/fontlist/font?ibm_model3x_alt1),
  - [Kaypro2K G-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?kaypro2k_g-2y),
  - [MBytePC230 Mono](https://int10h.org/oldschool-pc-fonts/fontlist/font?mbytepc230_mono),
  - [MBytePC230 CGA-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?mbytepc230_cga-2y),
  - [Phoenix BIOS-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?phoenix_bios-2y),
  - [ToshibaTxL2 8x16](https://int10h.org/oldschool-pc-fonts/fontlist/font?toshibatxl2_8x16),
  - [SperryPC CGA-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?sperrypc_cga-2y),
  - [SperryPC 8x16](https://int10h.org/oldschool-pc-fonts/fontlist/font?sperrypc_8x16),
  - [Tandy1K-I 200L-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?tandy1k-i_200l-2y),
  - [SanyoMBC16-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?sanyombc16-2y),
  - [OlivettiThin 8x16](https://int10h.org/oldschool-pc-fonts/fontlist/font?olivettithin_8x16),
  - [Nix8810 M35-2y](https://int10h.org/oldschool-pc-fonts/fontlist/font?nix8810_m35-2y),
- UI:
  - provide a minimal and beuatiful experience meshing a retro delivery and aesthetic, amplified by modern tooling.
  - we want the ui to fade upon use, and only appear during active hover, ADD TICK BOX TO TURN IT OFF AND OR IN `> settings:[ ] "PIN_UI: Turn Active UI Fade OFF"`
  - The Octobrachial display should provide a very similar experience VISUALLY to syntax highlighting, but it is not going to be strictly enforced. This is for visual and color output purposes.
  - **NOTE**: POPULATE OBVIOUS REQUIRED FUNCTIONS SUCH AS but not limited to: [import, export]
  - HOVER MENU -> `settings > create > category`: a menu in place for the user to create categories and set hotkeys to them.

> BELOW ARE RUDIMENTARY STARTERS, NOT FINAL ABSOLUTES. I MADE THIS FAST, AND WITH INTENT THAT THEY WOULD BE FURTHER OPTIMIZED AND OR DEVELOPED. **THEY ARE NOT COMPLETE EXAMPLES OR FORMATS**

```txt
[settings]:style: "Octobrachial Arm"
└─ [create]
│   └─ [classification_category]
│      │
│      │                         ┌─ hier(a).[_float_],
│      └─ [img]->[cata]->[hier]:[├─ hier(b).[_fav_],
│                                └─ hier(c).[1, 2, 3, 4, 5, 6, 7, 8]
│
[settings]:style: "Octobrachial Beak"
│ └─ [create]
│    └─ [classification_category]
│       │
│       │              ┌─ hier(a).[_float_],
│       └─ [images] -> │  hier(b).["_fav_"]
│                      └─ hier(b).[1, 2, 3, 4, 5, 6, 7, 8]
│
[settings]:style:"Boxtobrachial"
└─ [create]
   └─ [classification_category]
      │   ┌──────────────────────────────────┐
      ├───┤ hier(a).[_float_],               │
      │   │ hier(b).[1, 2, 3, 4, 5, 6, 7, 8] │
      │   └──────────────────────────────────┘
      │   ┌──────────────────────────────────┐
      └───┤ hier(a).[_float_],               │
          │ hier(b).[1, 2, 3, 4, 5, 6, 7, 8] │
          └──────────────────────────────────┘
```

The above explains a menu system containing nexting settings;

- create​: the area housing areas to create your own settings
- classification_category​ : What the parent folder title is classified as
- hier​: Hierarchy of the image classification categories
- (a)​: the highest in the ladder, this is an alphabetical id system.
- (b)​: the second in the hierarchal ladder
- NOTE: It is important to distinguish that hierarchal alphabetical labels are for organizational and modular future purposing. THEY ARE ADJACENT in the SAME DIRECTORY, A IS NOT NESTED IN B.

---

### MIRRORED VERTICAL OUTPUT FOR MAXIMIZED SPATIAL USAGE

FOR NESTED DIRECTORIES, THE OUTPUT WILL START TO GET VERBOSE AND OR LONG. TO CIRCUMVENT THIS, WE WILL INTRODUCED MIRRORED VERTICAL OUTPUT BASED ON AVAILABLE WINDOW SPACE AND HIERARCHY IN THE ASSIGNMENT. THIS WILL REQUIRE LIGHTING FAST ERROR DETECTION **ON CLICK**;

- WHY DO IT THIS WAY? BECAUSE ALMOST ALWAYS HIERARCHAL (a) ITEMS CAN BE ASSUMED TO FOLD UPWARDS IF THERE IS EVER ANY ERROR TRIPPING LIMITED WINDOW SPACE
- FOR THE OUTPUTTED ASCII COMPONENTS IN THE DISPLAY WINDOW. IF USER CLICKS AND THERE IS NOT SPACE, THAT TRIPS AN ERROR WHICH SHOULD MIRROR THE ELEMENT UPWARDS OR DOWWARDS BASED ON WHAT IS REQUIRED TO IMPROVE SPACE IN THE OUTPUT.
- THIS CAN EASILY BE DONE FAST WITH RUST, AND LIGHT. IT DOES NOT AT ALL NEED TO BE A HEAVY METHOD / FUNCTION. IT IS PART OF THE REASON WHY I NAMED IT `Octobrachial`; **WHEN CODE BECOMES DEEPY NESTED, AND THEIR IS SIMILAR BRANCHES, IT OFTEN TURNS INTO WHAT VISUALLY LOOKS LIKE AN OCTOPUS**
  - THAT IS THE NATURAL WAY OF THINGS, THE NATURAL OPERATION AND ORDER OF ORGANIZATIONS VISUAL OUTPUT; I WANT TO LEAN INTO THIS DOCTRINE AND DEVELOP INTO IT.

BASED ON ITS HIERARCHY; TO SAVE SPACE THE OUTPUT SHOULD DISPLAY IN ONE OF TWO WAYS
IN THEORY, A LARGER DIRECTORY WOULD START TO TAKE SHAPE AS AN OCTOPUS;

```txt
│                                ┌─ hier(a).[_float_], 
│      ┌─ [img]->[cata]->[hier]:[├─ hier(b).[_fav_],
│      │                         └─ hier(c).[1, 2, 3, 4, 5, 6, 7, 8]
│      │
│   ┌─ [classification_category]
│┌─ [create]
├[settings]:style: "Octobrachial Arm"
│
├[settings]:style: "Octobrachial Arm"
│└─ [create]
│   └─ [classification_category]
│     │
│     │                         ┌─ hier(a).[_float_],
│     └─ [img]->[cata]->[hier]:[├─ hier(b).[_fav_],
│                               └─ hier(c).[1, 2, 3, 4, 5, 6, 7, 8]
```

```jsonc
# Definitions are listed in their hierarchal order
 ##
  [
    "_fav_":"favourites, highest in the ladder for hierarchal",
    "_float_":"high traffic, high value directory",
    "classify":"Assign a color id and a target directory",
    "category":"title of directory, users will be able to create categories and deleted them through the 'create' setting in the menu",
 "hierarchal":"The alphabetic hierarchy in which the categories are prioritized"
    "hotkeys":"assigned_to:[category, cid]"
  ],
# Help and Settings
  [
    "h":"Help Menu",
    ".":"Settings"
  ],
# Hotkeys and color identifiers;  
  ## Classifier Keys on ACTIVE IMAGE WINDOW
  [                           
    "z​":"1"("cid":"#e56399​"), # z key Saves to category(directory) 1
    "x​":"2"("cid":"#92CC7C​"), # x key Saves to category(directory) 2
    "c​":"3"("cid":"#7f96ff"​), # c key Saves to category(directory) 3
    "v​":"4"("cid":"#a6cfd5​"), # v Saves to category(directory) 4 
    "SPACEBAR":"_float_"("cid":"#E6D463") # SPACEBAR Save to [_float_]
  ],
  ## Browse Keys on ACTIVE IMAGE WINDOW
  [
    "LEFT_ARROW​":"{last}<-[IMAGE]",
    "RIGHT_ARROW"​:"{next}->[IMAGE]",
    "UP_ARROW":​"{'skip':'back'(+5)::[IMAGE]}",
    "DOWN_ARROW"​: "{'skip':'next'}(+5)::[IMAGE]}"
    "LEFT_SQUARE_BRACKET"​:"{'skip':'next'}(+25)::[IMAGE]}",
    "RIGHT_SQUARE_BRACKET":​"{'skip':'back'(+25)::[IMAGE]}",
    "SEMICOLON"​:"{'skip':'next'}(+50)::[IMAGE]}",
    "APOSTROPHE":​"{'skip':'back'(+50)::[IMAGE]}"
  ]
```

## In Operation

It should move the image present in the browser to the correct category and assign its color id IN THE APP, this wont display in windows, this is for browser application display. The application browser​ is ultimaltely designed to help with big image and video directories for manual sorting(classifing)

## Inspiration Octobrachial

> [0] [Octobrachial_postal_worker](FICTIONAL CHARACTER FIRST SEEN AS A MULTI ARMED MAIL FLINGER FROM THE MEN IN BLACK SERIES IN THE MOVIE MEN IN BLACK II)

   1. The Movie: [Men in Black II](https://www.google.com/search?q=men+in+black+ii&kgmid=/m/031t2d) (2002).
   2. The Scene: The Truro, Massachusetts Post Office scene. It happens about 25 to 30 minutes into the film. [1, 2]
   3. The Context: It is the exact moment Agent J (Will Smith) goes to find a retired Agent K (Tommy Lee Jones), who is working as the local postmaster. [3, 4]
   4. The Action: Agent J walks up to the giant mechanical mail sorting console, opens the side panel door like a cabinet, and reveals the alien sitting inside. [5]
   5. The Visual Reference: The alien has eight arms flying everywhere sorting letters into slots at lightning speed while wearing a postal uniform shirt and casually smoking a cigarette. Agent K walks over, rips the cigarette out of his mouth, and tells him, "No smoking!". [5, 6]

>> Only official scene; [Post Office Mail Sorter Alien](https://www.youtube.com/watch?v=CcilBwPBFFU)

[1] [https://www.youtube.com](https://www.youtube.com/watch?v=W1_uTMFWfg4)
[2] [https://www.reddit.com](https://www.reddit.com/r/USPS/comments/uoz5z5/men_in_black_ii_post_office_aliens_scene_310/)
[3] [https://www.youtube.com](https://www.youtube.com/shorts/yfC8_QtqMwU)
[4] [https://www.youtube.com](https://www.youtube.com/watch?v=4HgUh5bOgbM)
[5] [https://www.imdb.com](https://www.imdb.com/title/tt0120912/quotes/)
[6] [https://aliens.fandom.com](https://aliens.fandom.com/wiki/Postal_Sorting_Alien%27s_Species)

/*
 * Dumps glyph outlines and advances from FreeType in crater's record format.
 *
 * Usage: ft_dump <fonts.txt> <ppem>...
 *
 * fonts.txt has one "<id> <path>" line per font. A ppem of 0 dumps in font
 * units. Records:
 *
 *   font <id>
 *   config <name>                unscaled, or ppem=<n>
 *   glyph <gid> <advance> <path>
 *   glyph <gid> error <code>
 *
 * Unscaled coordinates and advances are font units. Scaled coordinates are
 * 26.6 and scaled advances 16.16, both integers. A path is a sequence of
 * M x y, L x y, Q cx cy x y, C cx0 cy0 cx1 cy1 x y and Z, with Z closing
 * each contour.
 */
#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_OUTLINE_H
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int in_contour;

static int move_to(const FT_Vector* to, void* user) {
  if (in_contour) printf(" Z");
  printf(" M %ld %ld", to->x, to->y);
  in_contour = 1;
  return 0;
}

static int line_to(const FT_Vector* to, void* user) {
  printf(" L %ld %ld", to->x, to->y);
  return 0;
}

static int conic_to(const FT_Vector* c, const FT_Vector* to, void* user) {
  printf(" Q %ld %ld %ld %ld", c->x, c->y, to->x, to->y);
  return 0;
}

static int cubic_to(const FT_Vector* c0, const FT_Vector* c1, const FT_Vector* to,
                    void* user) {
  printf(" C %ld %ld %ld %ld %ld %ld", c0->x, c0->y, c1->x, c1->y, to->x, to->y);
  return 0;
}

static const FT_Outline_Funcs funcs = {move_to, line_to, conic_to, cubic_to, 0, 0};

static void dump_config(FT_Face face, int ppem) {
  FT_Int32 flags = FT_LOAD_NO_BITMAP | FT_LOAD_NO_HINTING | FT_LOAD_NO_AUTOHINT;
  if (ppem == 0) {
    printf("config unscaled\n");
    flags |= FT_LOAD_NO_SCALE | FT_LOAD_LINEAR_DESIGN;
  } else {
    printf("config ppem=%d\n", ppem);
    FT_Set_Char_Size(face, ppem * 64, ppem * 64, 72, 72);
  }
  for (FT_Long gid = 0; gid < face->num_glyphs; gid++) {
    FT_Error error = FT_Load_Glyph(face, gid, flags);
    if (error) {
      printf("glyph %ld error %d\n", gid, error);
      continue;
    }
    printf("glyph %ld %ld", gid, face->glyph->linearHoriAdvance);
    in_contour = 0;
    if (face->glyph->format == FT_GLYPH_FORMAT_OUTLINE)
      FT_Outline_Decompose(&face->glyph->outline, &funcs, NULL);
    if (in_contour) printf(" Z");
    printf("\n");
  }
}

int main(int argc, char** argv) {
  if (argc < 3) {
    fprintf(stderr, "usage: %s <fonts.txt> <ppem>...\n", argv[0]);
    return 2;
  }
  FT_Library library;
  if (FT_Init_FreeType(&library)) return 1;
  FILE* list = fopen(argv[1], "r");
  if (!list) return 1;
  char line[4096];
  while (fgets(line, sizeof line, list)) {
    line[strcspn(line, "\r\n")] = 0;
    char* space = strchr(line, ' ');
    if (!space) continue;
    *space = 0;
    const char* id = line;
    const char* path = space + 1;
    printf("font %s\n", id);
    FT_Face face;
    if (FT_New_Face(library, path, 0, &face)) {
      printf("error load\n");
      continue;
    }
    for (int i = 2; i < argc; i++) dump_config(face, atoi(argv[i]));
    FT_Done_Face(face);
  }
  fclose(list);
  FT_Done_FreeType(library);
  return 0;
}

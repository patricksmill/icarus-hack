#include <stdio.h>
#include "overrides.h"

/* Placeholder until the projection setup is found. Once FindProjection
   names it, set the address here and turn this on: it logs every call so
   the arguments can be matched to the game's camera, then runs the
   original unchanged. */
#if 0
#define PROJECTION_SETUP 0x00000000

RECOMP_OVERRIDE(PROJECTION_SETUP) {
    fprintf(stderr, "projection r0=%08X r1=%08X s0=%f s1=%f\n",
            ctx->r[0], ctx->r[1], vfp_from_s(ctx->vfp[0]), vfp_from_s(ctx->vfp[1]));
    CALL(RECOMP_ORIGINAL(PROJECTION_SETUP));
}
#endif

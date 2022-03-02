#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stddef.h>
#include <setjmp.h>
#include <unistd.h>

#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"
#include "ppport.h"

typedef uintptr_t usize;
typedef intptr_t isize;

extern usize RSPL_StackMark_count(usize self) {
    SV **ptr = PL_stack_base + self;
    if (ptr > PL_stack_sp) {
        return 0;
    }
    return PL_stack_sp - ptr;
}

extern SV* RSPL_stack_get(usize offset) {
    SV **ptr = PL_stack_base + offset;
    if (ptr > PL_stack_sp) {
        return NULL;
    }
    return *ptr;
}

extern void RSPL_croak_sv(SV *sv) {
    croak_sv(sv);
}

extern CV* RSPL_newXS_flags(
    const char *name,
    void *subaddr,
    const char *const filename,
    const char *const proto,
    uint32_t flags
) {
    return newXS_flags(name, subaddr, filename, proto, flags);
}

extern double RSPL_SvNV(SV *sv) {
    return SvNV(sv);
}

extern isize RSPL_SvIV(SV *sv) {
    return SvIV(sv);
}

extern const char* RSPL_SvPVutf8(SV *sv, size_t *out_len) {
    size_t length;
    const char *out = SvPVutf8(sv, length);
    *out_len = length;
    return out;
}

extern const char* RSPL_SvPV(SV *sv, size_t *out_len) {
    size_t length;
    const char *out = SvPV(sv, length);
    *out_len = length;
    return out;
}

extern bool RSPL_SvUTF8(SV *sv) {
    return SvUTF8(sv) != 0;
}

/// SvPVbyte with a downgrade check to avoid croaking!
extern const char* RSPL_SvPVbyte(SV *sv, size_t *out_len) {
    size_t length;
    if (!sv_utf8_downgrade(sv, true))
        return NULL;
    const char *out = SvPVbyte(sv, length);
    *out_len = length;
    return out;
}

extern SV* RSPL_sv_2mortal(SV *sv) {
    return sv_2mortal(sv);
}

extern SV* RSPL_get_undef() {
    return &PL_sv_undef;
}

extern SV* RSPL_get_yes() {
    return &PL_sv_yes;
}

extern SV* RSPL_get_no() {
    return &PL_sv_no;
}

extern usize RSPL_PL_markstack_ptr() {
    return *PL_markstack_ptr;
}

extern usize RSPL_pop_markstack_ptr() {
    return *PL_markstack_ptr--;
}

extern void RSPL_stack_shrink_to(usize count) {
    PL_stack_sp = PL_stack_base + count;
}

extern void RSPL_stack_resize_by(isize count) {
    if (count > 0) {
        isize space = PL_stack_max - PL_stack_sp;
        if (space < count) {
            Perl_stack_grow(aTHX_ PL_stack_sp, PL_stack_sp, count - space);
        }
    }
    PL_stack_sp += count;
}

extern SV** RSPL_stack_sp() {
    return PL_stack_sp;
}

extern SV* RSPL_newRV_inc(SV *rv) {
    return newRV_inc(rv);
}

extern SV* RSPL_newSViv(isize v) {
    return newSViv(v);
}

extern SV* RSPL_newSVuv(usize v) {
    return newSVuv(v);
}

extern SV* RSPL_newSVnv(double v) {
    return newSVnv(v);
}

extern SV* RSPL_newSVpvn(const char *v, size_t len) {
    return newSVpvn(v, len);
}

extern SV* RSPL_newSVpvn_utf8(const char *v, size_t len) {
    return newSVpvn_utf8(v, len, 1);
}

extern SV* RSPL_SvREFCNT_inc(SV *sv) {
    return SvREFCNT_inc(sv);
}

extern void RSPL_SvREFCNT_dec(SV *sv) {
    return SvREFCNT_dec(sv);
}

extern bool RSPL_is_scalar(SV *sv) {
    return SvTYPE(sv) < SVt_PVAV;
}

extern bool RSPL_SvOK(SV *sv) {
    return SvOK(sv);
}

extern bool RSPL_SvANY(SV *sv) {
    return SvANY(sv);
}

extern bool RSPL_SvTRUE(SV *sv) {
    return SvTRUE(sv);
}

// This must be the same as in rust!
#define TYPE_FLAG_INT     1
#define TYPE_FLAG_DOUBLE  2
#define TYPE_FLAG_STRING  4

static const uint32_t type_flags[16] = {
    [SVt_NULL] = 0,
    [SVt_IV] = TYPE_FLAG_INT,
    [SVt_NV] = TYPE_FLAG_INT | TYPE_FLAG_DOUBLE,
    [SVt_PV] = TYPE_FLAG_STRING,
    [SVt_PVIV] = TYPE_FLAG_STRING | TYPE_FLAG_INT,
    [SVt_PVNV] = TYPE_FLAG_STRING | TYPE_FLAG_INT | TYPE_FLAG_DOUBLE,
    [SVt_PVMG] = ~0,
};

extern bool RSPL_is_defined(SV *sv) {
    // see OP_DEFINED in pp_hot.c in perl code
    if (!sv || !SvANY(sv))
        return false;

    switch (SvTYPE(sv)) {
    case SVt_PVAV:
        return (
                AvMAX(sv) >= 0
                || SvGMAGICAL(sv)
                || (SvRMAGICAL(sv) && mg_find(sv, PERL_MAGIC_tied)));
    case SVt_PVHV:
        return (HvARRAY(sv)
                || SvGMAGICAL(sv)
                || (SvRMAGICAL(sv) && mg_find(sv, PERL_MAGIC_tied)));
    case SVt_PVCV:
        // we don't support subs at all
        return false;
    default:
        SvGETMAGIC(sv);
        return SvOK(sv);
    }
}

extern uint32_t RSPL_svtype(SV *sv) {
    return SvTYPE(sv);
}

extern uint32_t RSPL_type_flags(SV *sv) {
    return type_flags[SvTYPE(sv)];
}

extern bool RSPL_has_integer(SV *sv) {
    return 0 != (type_flags[SvTYPE(sv)] & TYPE_FLAG_INT);
}

extern bool RSPL_has_double(SV *sv) {
    return 0 != (type_flags[SvTYPE(sv)] & TYPE_FLAG_DOUBLE);
}

extern bool RSPL_has_string(SV *sv) {
    return 0 != (type_flags[SvTYPE(sv)] & TYPE_FLAG_STRING);
}

extern SV* RSPL_SvRV(SV *sv) {
    return SvRV(sv);
}

extern SV* RSPL_dereference(SV *sv) {
    return SvROK(sv) ? SvRV(sv) : NULL;
}

extern bool RSPL_is_reference(SV *sv) {
    return SvROK(sv);
}

extern bool RSPL_is_array(SV *sv) {
    return SvTYPE(sv) == SVt_PVAV;
}

extern bool RSPL_is_hash(SV *sv) {
    return SvTYPE(sv) == SVt_PVHV;
}

extern AV* RSPL_newAV() {
    return newAV();
}

extern usize RSPL_av_len(AV *av) {
    return av_len(av);
}

extern void RSPL_av_extend(AV *av, ssize_t len) {
    av_extend(av, len);
}

extern void RSPL_av_push(AV *av, SV *sv) {
    av_push(av, sv);
}

extern SV* RSPL_av_pop(AV *av) {
    return av_pop(av);
}

extern SV** RSPL_av_fetch(AV *av, ssize_t index, int32_t lval) {
    return av_fetch(av, index, lval);
}

extern HV* RSPL_newHV() {
    return newHV();
}

extern usize RSPL_HvTOTALKEYS(HV *hv) {
    return HvTOTALKEYS(hv);
}

extern SV** RSPL_hv_fetch(HV *hv, const char *key, int32_t klen, int32_t lval) {
    return hv_fetch(hv, key, klen, lval);
}

/// ALWAYS takes ownership of 'value'.
extern bool RSPL_hv_store(HV *hv, const char *key, int32_t klen, SV *value) {
    if (hv_store(hv, key, klen, value, 0) == NULL) {
        SvREFCNT_dec(value);
        return false;
    } else {
        return true;
    }
}

extern bool RSPL_hv_store_ent(HV *hv, SV *key, SV *value) {
    if (hv_store_ent(hv, key, value, 0) == NULL) {
        SvREFCNT_dec(value);
        return false;
    } else {
        return true;
    }
}

extern void RSPL_hv_iterinit(HV *hv) {
    hv_iterinit(hv);
}

extern SV* RSPL_hv_iternextsv(HV *hv, char **key, int32_t *retlen) {
    return hv_iternextsv(hv, key, retlen);
}

extern HE* RSPL_hv_iternext(HV *hv) {
    return hv_iternext(hv);
}

extern SV* RSPL_hv_iterkeysv(HE *he) {
    return hv_iterkeysv(he);
}

extern SV* RSPL_hv_iterval(HV *hv, HE *he) {
    return hv_iterval(hv, he);
}

extern HV* RSPL_gv_stashsv(SV *name, int32_t flags) {
    return gv_stashsv(name, flags);
}

extern SV* RSPL_sv_bless(SV *sv, HV *stash) {
    return sv_bless(sv, stash);
}

extern void RSPL_ENTER() {
    ENTER;
}

extern void RSPL_SAVETMPS() {
    SAVETMPS;
}

extern void RSPL_FREETMPS() {
    FREETMPS;
}

extern void RSPL_LEAVE() {
    LEAVE;
}

extern const char* RSPL_sv_reftype(const SV *const sv, const int ob) {
    return sv_reftype(sv, ob);
}

// We we don't need to generate the numeric value:
extern uint32_t RSPL_PVLV() {
    return SVt_PVLV;
}

extern SV* RSPL_LvTARG(SV *sv) {
    return LvTARG(sv);
}

/// Takes ownership of `orig`, returns an owned scalar.
/// This does NOT check `off` and `len`. That's up to the caller.
extern SV* RSPL_substr(SV *orig, usize off, usize len) {
    SV *ret = newSV_type(SVt_PVLV);
    sv_magic(ret, NULL, PERL_MAGIC_substr, NULL, 0);
    LvTYPE(ret) = 'x';
    LvTARG(ret) = orig;
    LvTARGOFF(ret) = off;
    LvTARGLEN(ret) = len;
    return ret;
}

// We prefer this unsigned.
//extern unsigned char RSPL_LvTYPE(SV *sv) {
//    return (unsigned char)LvTYPE(sv);
//}

//extern void RSPL_vivify_defelem(SV *sv) {
//    Perl_vivify_defelem(aTHX_ sv);
//}

//extern uint32_t RSPL_SvFLAGS(SV *sv) {
//    return SvFLAGS(sv);
//}

//extern bool RSPL_SvMAGICAL(SV *sv) {
//    return SvMAGICAL(sv);
//}

extern void RSPL_SvGETMAGIC(SV *sv) {
    return SvGETMAGIC(sv);
}

extern const MGVTBL* RSPL_vtbl_substr() {
    return &PL_vtbl_substr;
}

extern int RSPL_PERL_MAGIC_substr() {
    return PERL_MAGIC_substr;
}

/// Create a new all-zeroes vtbl as perl docs suggest this is the safest way to
/// make sure what a `PERL_MAGIC_ext` magic actually means, as the ptr value
/// may be arbitrary. So this function is actually used to allocate "tags".
extern MGVTBL* RSPL_mgvtbl_new() {
    return calloc(1, sizeof(MGVTBL));
}

extern MAGIC* RSPL_sv_magicext(
    SV *sv,
    SV *obj,
    int how,
    const MGVTBL *vtbl,
    const char *name,
    int32_t namelen)
{
    return sv_magicext(sv, obj, how, vtbl, name, namelen);
}

extern MAGIC* RSPL_mg_findext(const SV *sv, int ty, const MGVTBL *vtbl) {
    return mg_findext(sv, ty, vtbl);
}

/* it's not completely clear to me whether we want to use mg_freeext or sv_unmagicext
extern void RSPL_mg_freeext(SV *sv, int ty, const MGVTBL *vtbl) {
    return mg_freeext(sv, ty, vtbl);
}
*/
extern void RSPL_sv_unmagicext(SV *sv, int ty, MGVTBL *vtbl) {
    // always returns 0 currently
    sv_unmagicext(sv, ty, vtbl);
}

// Seems safer than depending on a `struct magic` declaration within rust code:
extern const MGVTBL* RSPL_MAGIC_virtual(const MAGIC* mg) {
    return mg->mg_virtual;
}

extern const char* RSPL_MAGIC_ptr(const MAGIC* mg) {
    return mg->mg_ptr;
}

extern isize RSPL_MAGIC_len(const MAGIC* mg) {
    return mg->mg_len;
}

extern int RSPL_PERL_MAGIC_ext() {
    return PERL_MAGIC_ext;
}

extern HV* RSPL_defstash() {
    return PL_defstash;
}

/*
These make are convoluted brainfarts:
        SVt_NULL                 undef
        SVt_IV                   all the above or int
        SVt_NV                   all the above or a double
        SVt_PV                   undef or a string
        SVt_PVIV                 PV or IV
        SVt_PVNV                 PV or NV
        SVt_PVMG                 all of the above with tentacles, 2 heads and unicorn poop on top

These make some sense
        SVt_INVLIST               Bleeding smelly perl guts
        SVt_REGEXP                Sandpaper
        SVt_PVGV                  Typeglob
        SVt_PVLV                  C++ style reference to another scalar (implicit deref)

These make sense
        SVt_PVAV                  Arrays
        SVt_PVHV                  Hashes
        SVt_PVCV                  Subroutine
        SVt_PVFM                  Formats
        SVt_PVIO                  I/O objects
*/

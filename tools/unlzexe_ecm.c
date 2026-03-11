/* unlzexe.c
* unlzexe ecm release 4, public domain
* based on unlzexe ver 0.5 (PC-VAN UTJ44266 Kou )
*
*   UNLZEXE converts the compressed file by lzexe
*   (ver.0.90, 0.91, LZE1, LZE2, LZE3, LZE4, LZE5, LZE6, LZX0) to the
*   UNcompressed executable one.
*
*   usage:  UNLZEXE packedfile[.EXE] [unpackedfile.EXE]
*/

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>
#define FAILURE 1
#define SUCCESS 0

#ifdef __DOS__
#include <dos.h>
#else
 #define stricmp strcasecmp
#endif

typedef uint32_t DWORD;
typedef uint16_t WORD;
typedef uint8_t BYTE;

int isjapan(void);
int japan_f;
#define	iskanji(c)	( ('\x81'<=(c)&&(c)<='\x9f') || ('\xe0'<=(c)&&(c)<='\xfc') )

static unsigned debugging = 0;
static unsigned dotmode = 0;
static DWORD outsize = 0, imagesize = 0;
static WORD ihead[0x10],ohead[0x10],inf[8],reloctable,scratch;

char *tmpfname = "$tmpfil$.exe";
char *backup_ext = ".olz";
char ipath[FILENAME_MAX],
     opath[FILENAME_MAX],
     ofname[13];

char* fields[] = {
	"Signature", "Extra Bytes", "Pages", "Reloc Items",
	"Header Size", "Min Alloc", "Max Alloc", "Init SS",
	"Init SP", "Checksum", "Init IP", "Init CS",
	"Reloc Table", "Overlay Num", 0 };

char variname_lenprog[] = "lenprog";
BYTE patterns_lenprog[] = { 0x0E, 0x1F, 0xB9, 0x00, 0x00, 0x89, 0xCE };
BYTE wildcard_lenprog[] = { 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00 };
			/* push cs, pop ds, mov cx imm, mov si cx */
WORD varibase_lenprog = 3;
WORD* address_lenprog = &inf[6];
WORD found_lenprog = -1;

char variname_decalage[] = "decalage";
BYTE patterns_decalage[] = { 0x8C, 0xDB, 0x81, 0xC3, 0x00, 0x00, 0x8E, 0xC3 };
BYTE wildcard_decalage[] = { 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00 };
			/* mov bx ds, add bx imm16, mov es bx */
WORD varibase_decalage = 4;
WORD* address_decalage = &inf[5];
WORD found_decalage = -1;

char variname_lenlz[] = "lenlz";
BYTE patterns_lenlz[] = { 0xBD, 0x00, 0x00, 0x8C, 0xDA, 0x89, 0xE8 };
BYTE wildcard_lenlz[] = { 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00 };
			/* mov bp imm, mov dx ds, mov ax bp */
WORD varibase_lenlz = 1;
WORD* address_lenlz = &inf[4];
WORD found_lenlz = -1;

char variname_reloctable[] = "reloctable";
BYTE patterns_reloctable[] = { 0x0E, 0x1F, 0xBE, 0x00, 0x00, 0x5B, 0x83, 0xC3, 0x10 };
BYTE wildcard_reloctable[] = { 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00 };
			/* push cs, pop ds, mov si imm, pop bx, add bx 10h */
WORD varibase_reloctable = 3;
WORD* address_reloctable = &reloctable;
WORD found_reloctable = -1;

char variname_sp[] = "sp";
BYTE patterns_sp[] = { 0xBF, 0x00, 0x00, 0xBE, 0x00, 0x00, 0x01, 0xC6 };
BYTE wildcard_sp[] = { 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0x00 };
			/* mov di imm, mov si imm, add si ax */
WORD varibase_sp = 1;
WORD* address_sp = &ohead[8];
WORD found_sp = -1;

char variname_ss[] = "ss";
BYTE patterns_ss[] = { 0xBF, 0x00, 0x00, 0xBE, 0x00, 0x00, 0x01, 0xC6 };
BYTE wildcard_ss[] = { 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0x00 };
			/* mov di imm, mov si imm, add si ax */
WORD varibase_ss = 4;
WORD* address_ss = &ohead[7];
WORD found_ss = -1;

char variname_ip[] = "ip";
BYTE patterns_ip[] = { 0xFA, 0x8E, 0xD6, 0x89, 0xFC, 0xFB, 0xEA, 0,0,0,0 };
BYTE wildcard_ip[] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,0xFF,0xFF,0xFF };
			/* cli, mov ss si, mov sp di, sti, jmp far imm */
WORD varibase_ip = 7;
WORD* address_ip = &ohead[0xA];
WORD found_ip = -1;

char variname_cs[] = "cs";
BYTE patterns_cs[] = { 0xFA, 0x8E, 0xD6, 0x89, 0xFC, 0xFB, 0xEA, 0,0,0,0 };
BYTE wildcard_cs[] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,0xFF,0xFF,0xFF };
			/* cli, mov ss si, mov sp di, sti, jmp far imm */
WORD varibase_cs = 9;
WORD* address_cs = &ohead[0xB];
WORD found_cs = -1;

char variname_getbit[] = "getbit function";
BYTE patterns_getbit[] = { 0xD1, 0xED, 0x4A, 0x75, 0x04, 0xAD, 0x95, 0xB2, 0x10, 0xC3 };
BYTE wildcard_getbit[] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 };
			/* shr bp 1, dec dx, jnz, lodsw, xchg bp ax, mov dh 10h, retn */
WORD varibase_getbit = 0;
WORD* address_getbit = &scratch;
WORD found_getbit = -1;

char variname_segchange[] = "segment change";
BYTE patterns_segchange[] = { 0x8C, 0xC0, 0x01, 0xD8, 0x2D, 0x00, 0x02, 0x8E, 0xC0 };
BYTE wildcard_segchange[] = { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 };
			/* mov ax es, add ax bx, sub ax 200h, mov es ax */
WORD varibase_segchange = 0;
WORD* address_segchange = &scratch;
WORD found_segchange = -1;

/*
char variname_[] = "";
BYTE patterns_[] = { 0x, 0x, 0x, 0x, 0x, 0x, 0x };
BYTE wildcard_[] = { 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00 };
WORD varibase_ = 3;
WORD* address_ = &inf[];
WORD found_ = -1;
*/

#define TABLEITEM(name, needX0) \
	{ variname_ ## name, \
	  patterns_ ## name, \
	  wildcard_ ## name, \
	  &varibase_ ## name, \
	  &address_ ## name, \
	  &found_ ## name, \
	  needX0, \
	  sizeof(patterns_ ## name), \
	}

struct patterntable {
	char* name;
	BYTE* patterns;
	BYTE* wildcard;
	WORD* varibase;
	WORD** address;
	WORD* found;
	unsigned needX0;
	WORD length;
	};
struct patterntable patterns[] = {
	TABLEITEM(ip, 1),
	TABLEITEM(cs, 1),
	TABLEITEM(sp, 1),
	TABLEITEM(ss, 1),
	TABLEITEM(lenlz, 1),
	TABLEITEM(decalage, 2),
	TABLEITEM(lenprog, 2),
	TABLEITEM(reloctable, 0),
	TABLEITEM(getbit, 0),
	TABLEITEM(segchange, 0),
};

BYTE getbyte(FILE* f) {
	return fgetc(f);
}
WORD getword(FILE* f) {
	WORD w;
	w = getbyte(f);
	w += getbyte(f) * 256;
	return w;
}
void putbyte(BYTE v, FILE* f) {
	fputc(v, f);
}
void putword(WORD w, FILE* f) {
	putbyte(w & 255, f);
	putbyte(w / 256, f);
}

WORD getwordfromarray(BYTE* p) {
	WORD w;
	w = p[0];
	w += p[1] * 256;
	return w;
}

int main(int argc,char **argv){
    int fnamechk(char*,char*,char*,int,char**);
    int  fnamechg(char*,char*,char*,int);
    int rdhead(const char*,FILE *,int *);
    int mkreltbl(FILE *,FILE *,int);
    int unpack(FILE *,FILE *);
    int wrhead(FILE *);
    
    FILE *ifile = NULL;
    FILE *ofile = NULL;
    DWORD insize;
    int  ver,rename_sw=0;
    char * env;

    if ( (env = getenv("DEBUG")) ) {
        debugging = atoi(env);
    }

    printf("UNLZEXE ecm release 4, public domain\n");
    japan_f=isjapan();
    if(argc!=3 && argc!=2){
        printf("usage: UNLZEXE packedfile [unpackedfile|.]\n");
        printf("\nSupported formats: LZEXE v0.90, v0.91, LZE1, LZE2, LZE3, LZE4, LZE5, LZE6, LZX0\n");
        exit(EXIT_FAILURE);
    }
    if(argc==2)
        rename_sw=1;
    if (argc == 3 && 0 == strcmp(argv[2], ".")) {
        printf("dot mode\n");
        dotmode = 1;
    }
    if(fnamechk(ipath,opath,ofname,argc,argv)!=SUCCESS) {
        exit(EXIT_FAILURE);
    }
    if((ifile=fopen(ipath,"rb"))==NULL){
        printf("'%s' :not found\n",ipath);
            exit(EXIT_FAILURE);
    }
    
    if(rdhead(ipath,ifile,&ver)!=SUCCESS){
        fclose(ifile); exit(EXIT_FAILURE);
    }
    if (debugging & 9) {
        unsigned ii;
        printf("Input file executable header fields:\n");
        for (ii = 0; fields[ii]; ++ii) {
            printf("%-16s %04Xh %u\n", fields[ii], ihead[ii], ihead[ii]);
        }
    }
    if(dotmode == 0 && (ofile=fopen(opath,"w+b"))==NULL){
        printf("can't open '%s'.\n",opath);
        fclose(ifile); exit(EXIT_FAILURE);
    }
    printf("file '%s' is compressed by LZEXE Ver. ",ipath);
    switch(ver){
    case 90: printf("0.90\n"); break;
    case 91: printf("0.91\n"); break;
    case 0xE1: printf("LZE1\n"); break;
    case 0xE2: printf("LZE2\n"); break;
    case 0xE3: printf("LZE3\n"); break;
    case 0xE4: printf("LZE4\n"); break;
    case 0xE5: printf("LZE5\n"); break;
    case 0xE6: printf("LZE6\n"); break;
    case 0xF0: printf("LZX0\n"); break;
    }
    if(mkreltbl(ifile,ofile,ver)!=SUCCESS) {
        fclose(ifile);
        if (!dotmode) fclose(ofile);
        remove(opath);
        exit(EXIT_FAILURE);
    }
    if(unpack(ifile,ofile)!=SUCCESS) {
        fclose(ifile);
        if (!dotmode) fclose(ofile);
        remove(opath);
        exit(EXIT_FAILURE);
    }
    fseek(ifile, 0, SEEK_END);
    insize = ftell(ifile);
    fclose(ifile);
    if (wrhead(ofile) != SUCCESS) {
        if (!dotmode) fclose(ofile);
        remove(opath);
        exit(EXIT_FAILURE);
    }
    if (!dotmode) {
        fclose(ofile);
        if(fnamechg(ipath,opath,ofname,rename_sw)!=SUCCESS){
            exit(EXIT_FAILURE);
        }
    }
    if (debugging & 17) {
        printf("Input file size:  %04"PRIX32"h %"PRIu32"\n",
        	insize, insize);
        printf("Output file size: %04"PRIX32"h %"PRIu32"\n",
        	outsize, outsize);
    }
    exit(EXIT_SUCCESS);
}



void parsepath(char *pathname, int *fname, int *ext);

/* file name check */
int fnamechk(char *ipath,char *opath, char *ofname,
              int argc,char **argv) {
    int idx_name,idx_ext;
    
    strcpy(ipath,argv[1]);
    parsepath(ipath,&idx_name,&idx_ext);
    if (! ipath[idx_ext]) strcpy(ipath+idx_ext,".exe");
    if(! stricmp(ipath+idx_name,tmpfname)){
        printf("'%s':bad filename.\n",ipath);
        return(FAILURE);
    }
    if (! dotmode) {
        if(argc==2)
            strcpy(opath,ipath);
        else
            strcpy(opath,argv[2]);
        parsepath(opath,&idx_name,&idx_ext);
        if (! opath[idx_ext]) strcpy(opath+idx_ext,".exe");
        if (!stricmp(opath+idx_ext,backup_ext)){
            printf("'%s':bad filename.\n",opath);
            return(FAILURE);
        }
        strncpy(ofname,opath+idx_name,12);
        strcpy(opath+idx_name,tmpfname);
    }
    return(SUCCESS);
}


int fnamechg(char *ipath,char *opath,char *ofname,int rename_sw) {
    int idx_name,idx_ext;
    char tpath[FILENAME_MAX];
    
    if(rename_sw) {
        strcpy(tpath,ipath);
        parsepath(tpath,&idx_name,&idx_ext);
        strcpy(tpath+idx_ext,backup_ext);
        remove(tpath);
        if(rename(ipath,tpath)){
            printf("can't make '%s'.\n", tpath);
            remove(opath);
            return(FAILURE);
        }
	printf("'%s' is renamed to '%s'.\n",ipath,tpath);
    }
    strcpy(tpath,opath);
    parsepath(tpath,&idx_name,&idx_ext);
    strcpy(tpath+idx_name,ofname);
    remove(tpath);
    if(rename(opath,tpath)){
        if(rename_sw) {
            strcpy(tpath,ipath);
            parsepath(tpath,&idx_name,&idx_ext);
            strcpy(tpath+idx_ext,backup_ext);
            rename(tpath,ipath);
        }
        printf("can't make '%s'.  unpacked file '%s' is remained.\n",
                 tpath, tmpfname);
        
        return(FAILURE);
    }
    printf("unpacked file '%s' is generated.\n",tpath);
    return(SUCCESS);
}

int isjapan() {
#ifdef __DOS__
    union REGS r;
    struct SREGS rs;
    BYTE buf[34];
    
    segread(&rs);
    rs.ds=rs.ss;  r.x.dx=(WORD)buf;
    r.x.ax=0x3800;
    intdosx(&r,&r,&rs);
    return(!strcmp((char *)buf+2,"\\"));
#else
	return 0;
#endif
}

void parsepath(char *pathname, int *fname, int *ext) {
    /* use  int japan_f */
    char c;
    int i;
    
    *fname=0; *ext=0;
    for(i=0; (c=pathname[i]); i++) {
        if(japan_f && iskanji(c)) 
            i++;
        else
            switch(c) {
            case ':' :
            case '/' :
            case '\\':  *fname=i+1; break;
            case '.' :  *ext=i; break;
            default  :  ;
            }
    }
    if(*ext<=*fname) *ext=i;
}
/*-------------------------------------------*/
/* static WORD allocsize; */
static long loadsize;
static WORD allocdelta = 0xFFFF;

int readwords(WORD* array, int amount, FILE * f) {
	int ii;
	for (ii = 0; ii < amount; ++ii) {
		array[ii] = getword(f);
	        if (ferror(f)) { return 0; }
	}
	return amount;
}

int writewords(WORD* array, int amount, FILE * f) {
	int ii;
	for (ii = 0; ii < amount; ++ii) {
		putword(array[ii], f);
	        if (ferror(f)) { return 0; }
	}
	return amount;
}

/* EXE header test (is it LZEXE file?) */
int rdhead(const char *ipath,FILE *ifile ,int *ver){
    if(readwords(ihead, 0x10, ifile) != 0x10) {
        printf("'%s' is not LZEXE file.\n",ipath);
        return FAILURE;
    }
    memcpy(ohead,ihead,sizeof ihead[0] * 0x10);
    /*
     * Some LZEXE 0.91-family variants keep a larger DOS header than the
     * canonical 2-paragraph layout while still advertising the normal LZ marker.
     * Esterian Conquest's binaries use a 0x20-paragraph header.
     */
    if(ihead[0]!=0x5a4d || ihead[4] < 2 || ihead[0x0d]!=0) {
        printf("'%s' is not LZEXE file.\n",ipath);
        return FAILURE;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZ09",4)==0){
        *ver=90; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZ91",4)==0){
        *ver=91; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE1",4)==0){
        *ver=0xE1; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE2",4)==0){
        *ver=0xE2; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE3",4)==0){
        *ver=0xE3; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE4",4)==0){
        *ver=0xE4; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE5",4)==0){
        *ver=0xE5; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZE6",4)==0){
        *ver=0xE6; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZX0",4)==0){
        *ver=0xF0; return SUCCESS ;
    }
    if(ihead[0x0c]==0x1c && memcmp(&ihead[0x0e],"LZ",2)==0){
        char s[5];
        memcpy(s, &ihead[0x0E], 4);
        s[4] = 0;
        if (s[2] < 32 || s[2] >= 0x7F || s[3] < 32 || s[3] >= 0x7F) {
            strcpy(s, "??");
        }
        printf("'%s' is compressed with an unknown LZEXE version %s\n",ipath,s);
        return FAILURE;
    }
    printf("'%s' is not LZEXE file.\n",ipath);
    return FAILURE;
}

int reloc90(FILE *ifile,FILE *ofile,long fpos);
int reloc91(FILE *ifile,FILE *ofile,long fpos);
int relocE1(WORD tableaddress,FILE *ifile,FILE *ofile,long fpos);

/* make relocation table */
int mkreltbl(FILE *ifile,FILE *ofile,int ver) {
    BYTE stub[512];
    WORD stubsize;
    long fpos;
    int i;

    /* allocsize=((ihead[1]+16-1)>>4) + ((ihead[2]-1)<<5) - ihead[4] + ihead[5]; */
    fpos=(long)(ihead[0x0b]+ihead[4])<<4;		/* goto CS:0000 */
    fseek(ifile,fpos,SEEK_SET);
    if (ver == 90 || ver == 91) {
        stubsize = 0;
        if (readwords(inf, 8, ifile) != 8) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        ohead[0x0a]=inf[0];		/* IP */
        ohead[0x0b]=inf[1];		/* CS */
        ohead[0x08]=inf[2];		/* SP */
        ohead[0x07]=inf[3];		/* SS */
    /* inf[4]:size of compressed load module (PARAGRAPH)*/
    /* inf[5]:increase of load module size (PARAGRAPH)*/
    /* inf[6]:size of decompressor with  compressed relocation table (BYTE) */
    /* inf[7]:check sum of decompresser with compressd relocation table(Ver.0.90) */
    } else if (ver == 0xE1) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < 0x12F
          || stub[0x006 - 2] != 0xB9
          || stub[0x010 - 2] != 0x81
          || stub[0x011 - 2] != 0xC3
          || stub[0x01B] != 0xBD
          || stub[0x110] != 0xBF
          || stub[0x113] != 0xBE
          || stub[0x116] != 0x01
          || stub[0x117] != 0xC6
          || stub[0x12A] != 0xEA
          || ihead[0x0A] != 0
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012 - 2]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007 - 2]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0x12B]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0x12D]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0x111]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0x114]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C]);	/* lenlz */
        reloctable = 0x12F;	/* 0x12F=compressed relocation table address */
    } else if (ver == 0xE2) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < (0x12F + 2)
          || stub[0x006] != 0xB9
          || stub[0x010] != 0x81
          || stub[0x011] != 0xC3
          || stub[0x01B + 2] != 0xBD
          || stub[0x110 + 2] != 0xBF
          || stub[0x113 + 2] != 0xBE
          || stub[0x116 + 2] != 0x01
          || stub[0x117 + 2] != 0xC6
          || stub[0x12A + 2] != 0xEA
          || ihead[0x0A] != 2
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0x12B + 2]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0x12D + 2]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0x111 + 2]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0x114 + 2]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C + 2]);	/* lenlz */
        allocdelta =getwordfromarray(&stub[0]);
        reloctable = 0x131;
    } else if (ver == 0xE3) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < (0x12F + 2 + 2)
          || stub[0x006] != 0xB9
          || stub[0x010] != 0x81
          || stub[0x011] != 0xC3
          || stub[0x01B + 2] != 0xBD
          || stub[0x110 + 2] != 0xBF
          || stub[0x113 + 2] != 0xBE
          || stub[0x116 + 2] != 0x01
          || stub[0x117 + 2] != 0xC6
          || stub[0x12A + 2 + 2] != 0xEA
          || ihead[0x0A] != 2
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0x12B + 2 + 2]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0x12D + 2 + 2]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0x111 + 2]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0x114 + 2]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C + 2]);	/* lenlz */
        allocdelta =getwordfromarray(&stub[0]);
        reloctable = 0x133;
    } else if (ver == 0xE4) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < (0x0FC + 5)
          || stub[0x006] != 0xB9
          || stub[0x010] != 0x81
          || stub[0x011] != 0xC3
          || stub[0x01B + 2] != 0xBD
          || stub[0x0E0] != 0xBF
          || stub[0x0E3] != 0xBE
          || stub[0x0E6] != 0x01
          || stub[0x0E7] != 0xC6
          || stub[0x0FC] != 0xEA
          || ihead[0x0A] != 2
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0xFD]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0xFF]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0xE1]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0xE4]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C + 2]);	/* lenlz */
        allocdelta =getwordfromarray(&stub[0]);
        reloctable = 0;
    } else if (ver == 0xE5) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < (0x10D + 5)
          || stub[0x006] != 0xB9
          || stub[0x010] != 0x81
          || stub[0x011] != 0xC3
          || stub[0x01B + 2] != 0xBD
          || stub[0x0F1] != 0xBF
          || stub[0x0F4] != 0xBE
          || stub[0x0F7] != 0x01
          || stub[0x0F8] != 0xC6
          || stub[0x10D] != 0xEA
          || ihead[0x0A] != 2
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0x10E]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0x110]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0xF2]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0xF5]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C + 2]);	/* lenlz */
        allocdelta =getwordfromarray(&stub[0]);
        reloctable = 0x112;
    } else if (ver == 0xE6) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < (0x0DB + 5)
          || stub[0x006] != 0xB9
          || stub[0x010] != 0x81
          || stub[0x011] != 0xC3
          || stub[0x01B + 2] != 0xBD
          || stub[0x0BF] != 0xBF
          || stub[0x0C2] != 0xBE
          || stub[0x0C5] != 0x01
          || stub[0x0C6] != 0xC6
          || stub[0x0DB] != 0xEA
          || ihead[0x0A] != 2
          ) {
            printf("error, invalid stub.\n");
            return (FAILURE);
        }
        inf[5]     =getwordfromarray(&stub[0x012]);	/* decalage */
        inf[6]     =getwordfromarray(&stub[0x007]);	/* lenprog */
        ohead[0x0a]=getwordfromarray(&stub[0x0DC]);	/* IP */
        ohead[0x0b]=getwordfromarray(&stub[0x0DE]);	/* CS */
        ohead[0x08]=getwordfromarray(&stub[0xC0]);	/* SP */
        ohead[0x07]=getwordfromarray(&stub[0xC3]);	/* SS */
        inf[4]     =getwordfromarray(&stub[0x01C + 2]);	/* lenlz */
        allocdelta =getwordfromarray(&stub[0]);
        reloctable = 0;
    } else if (ver == 0xF0) {
        stubsize = fread(stub, 1, 512, ifile);
        if (stubsize < 0x80 || ihead[0x0A] > 2 || ihead[0x0A] == 1) {
            printf("error, invalid LZX0 stub.\n");
            return (FAILURE);
        }
        if (ihead[0x0A] == 2) {
           allocdelta = getwordfromarray(&stub[0]);
        }
        reloctable = 0;
    } else {
        printf("internal error, invalid format version.\n");
        return (FAILURE);
    }
    if (ver == 0xF0 || ((debugging & 5) && ver >= 0xE1 && ver <= 0xE6)) {
/*
struct patterntable {
	char* name;
	BYTE* patterns;
	BYTE* wildcard;
	WORD* varibase;
	WORD** address;
	WORD* found;
	WORD length;
	};
struct patterntable patterns[] = {
*/
	struct patterntable* pp;
	BYTE searchpattern[32];
	WORD searchlength, ii, base, jj, left, found, value, abort = 0;
	for (ii = 0; ii < (sizeof(patterns)/sizeof(patterns[0])); ++ ii) {
	    pp = &patterns[ii];
	    searchlength = pp->length;
	    if (searchlength > sizeof(searchpattern)) {
	        printf("internal error, too long search pattern \"%s\".\n",
	        	pp->name);
	        return (FAILURE);
	    }
	    memcpy(searchpattern, pp->patterns, searchlength);
	    found = 0;
	    for (left = stubsize - searchlength + 1, base = 0;
	    	left > 0; -- left, ++ base) {
	    	for (jj = 0; jj < searchlength; ++ jj) {
	    	    if (pp->wildcard[jj]) {
	    	        searchpattern[jj] = stub[base + jj];
	    	    }
	    	}
	    	if (memcmp(stub + base, searchpattern, searchlength) == 0) {
	    	    found = 1;
	    	    break;
	    	}
	    }
	    if (found) {
	    	value = getwordfromarray(stub + base + *(pp->varibase));
	    	*(pp->found) = base + *(pp->varibase);
	    	if (debugging & 5) {
	    	 if (*pp->address == &scratch)
	    	  printf("at %04Xh,"
	    		" found \"%s\"\n",
	    		(unsigned)base,
	    		pp->name
	    		);
	    	 else if (ver != 0xF0)
	    	  printf("at %04Xh+%u=%04Xh,"
	    		" reads %04Xh (old %04Xh%s)"
	    		" found \"%s\"\n",
	    		(unsigned)base, (unsigned)*(pp->varibase),
	    		(unsigned)*(pp->found),
	    		(unsigned)value, (unsigned)**(pp->address),
	    		value != **(pp->address) ? ", mismatch!" : "",
	    		pp->name
	    		);
	    	 else if (ver == 0xF0)
	    	  printf("at %04Xh+%u=%04Xh,"
	    		" reads %04Xh"
	    		" found \"%s\"\n",
	    		(unsigned)base, (unsigned)*(pp->varibase),
	    		(unsigned)*(pp->found),
	    		(unsigned)value,
	    		pp->name
	    		);
	        }
	    	if (ver == 0xF0) {
	    	  **(pp->address) = value;
	    	}
	    } else {
	    	if ( (debugging & 5)
	    	  || (ver == 0xF0 && pp->needX0 == 1)
	    	  || (ver == 0xF0 && pp->needX0 == 2 && allocdelta == 0xFFFF)
	    	  ) {
	    	    if ((ver == 0xF0 && pp->needX0 == 1) ||
		      (ver == 0xF0 && pp->needX0 == 2 && allocdelta == 0xFFFF)) {
			abort = 1;
			printf("error: ");
		    }
	    	  printf("did not find \"%s\"\n", pp->name);
	    	}
	    }
	}
	if (abort) {
	    return (FAILURE);
	}
	if (ver == 0xF0) {
	    if (found_reloctable == 0xFFFF) {
	    	if (stubsize != found_cs + 2) {
	    	    printf("error: LZX0 no relocation stub invalid size"
	    	    	" (stubsize=%04Xh found_cs+2=%04Xh)"
	    	    	"\n", (unsigned)stubsize, (unsigned)found_cs + 2);
		    return (FAILURE);
	    	}
	    } else {
	    	if (stubsize < reloctable + 3
	    	  || reloctable != found_cs + 2) {
	    	    printf("error: LZX0 relocation stub invalid size"
	    	    	" (stubsize=%04Xh reloctable=%04Xh found_cs+2=%04Xh)"
	    	    	"\n",
	    	    	(unsigned)stubsize,
	    	    	(unsigned)reloctable,
	    	    	(unsigned)found_cs + 2);
		    return (FAILURE);
	    	}
	    }
	}
    }
    ohead[0x0c]=0x1c;		/* start position of relocation table */
    outsize = 0x1C;
    if (!dotmode) fseek(ofile,0x1cL,SEEK_SET);
    switch(ver){
    case 90: i=reloc90(ifile,ofile,fpos);
             break;
    case 91: i=reloc91(ifile,ofile,fpos);
             break;
    case 0xE1: i=relocE1(reloctable,ifile,ofile,fpos);
             break;
    case 0xE2: i=relocE1(reloctable,ifile,ofile,fpos);
             break;
    case 0xE3: i=relocE1(reloctable,ifile,ofile,fpos);
             break;
    case 0xE4:
    case 0xE6:
             ohead[3] = 0;
             i = SUCCESS;
             break;
    case 0xE5: i=relocE1(reloctable,ifile,ofile,fpos);
             break;
    case 0xF0:
	if (found_reloctable != 0xFFFF) {
	     i=relocE1(reloctable,ifile,ofile,fpos);
	     break;
	} else {
             ohead[3] = 0;
             i = SUCCESS;
             break;
        }
    default: i=FAILURE; break;
    }
    if(i!=SUCCESS){
        printf("error at relocation table.\n");
        return (FAILURE);
    }
    if (!dotmode) {
        fpos=ftell(ofile);
        if (fpos != outsize) {
            printf("error: outsize != fpos\n");
        }
    } else {
        fpos = outsize;
    }
    i=fpos & 0x1ff;
    if(i) i=0x200-i;
    ohead[4]=(fpos+i)>>4;
    for( ; i>0; i--) {
        outsize += 1;
        if (!dotmode) putbyte(0, ofile);
    }
    return(SUCCESS);
}
/* for LZEXE ver 0.90 */
int reloc90(FILE *ifile,FILE *ofile,long fpos) {
    unsigned int c;
    WORD rel_count=0;
    WORD rel_seg,rel_off;

    fseek(ifile,fpos+0x19d,SEEK_SET); 
    				/* 0x19d=compressed relocation table address */
    rel_seg=0;
    do{
        if(feof(ifile) || ferror(ifile) || (!dotmode && ferror(ofile))) return(FAILURE);
        c=getword(ifile);
        for(;c>0;c--) {
            rel_off=getword(ifile);
            outsize += 4;
            if (!dotmode) {
                putword(rel_off,ofile);
                putword(rel_seg,ofile);
            }
            rel_count++;
        }
        rel_seg += 0x1000;
    } while(rel_seg!=(0xf000+0x1000));
    ohead[3]=rel_count;
    return(SUCCESS);
}
/* for LZEXE ver 0.91*/
int reloc91(FILE *ifile,FILE *ofile,long fpos) {
    WORD span;
    WORD rel_count=0;
    WORD rel_seg,rel_off;

    fseek(ifile,fpos+0x158,SEEK_SET);
    				/* 0x158=compressed relocation table address */
    rel_off=0; rel_seg=0;
    for(;;) {
        if(feof(ifile) || ferror(ifile) || (!dotmode && ferror(ofile))) return(FAILURE);
        if((span=getbyte(ifile))==0) {
            span=getword(ifile);
            if(span==0){
                rel_seg += 0x0fff;
                continue;
            } else if(span==1){
                break;
            }
        }
        rel_off += span;
        rel_seg += (rel_off & ~0x0f)>>4;
        rel_off &= 0x0f;
        outsize += 4;
        if (!dotmode) {
            putword(rel_off,ofile);
            putword(rel_seg,ofile);
        }
        rel_count++;
    }
    ohead[3]=rel_count;
    return(SUCCESS);
}

/* for LZE1 */
int relocE1(WORD tableaddress,FILE *ifile,FILE *ofile,long fpos) {
    WORD span;
    WORD rel_count=0;
    WORD rel_seg,rel_off;

    fseek(ifile,fpos+tableaddress,SEEK_SET);
    rel_off=0; rel_seg=0;
    for(;;) {
        if(feof(ifile) || ferror(ifile) || (!dotmode && ferror(ofile))) return(FAILURE);
        if((span=getbyte(ifile))==255) {
            span=getword(ifile);
            if(span==0){
                rel_seg += 0x0fff;
                continue;
            } else if(span==1){
                break;
            }
        }
        rel_off += span;
        rel_seg += (rel_off & ~0x0f)>>4;
        rel_off &= 0x0f;
        outsize += 4;
        if (!dotmode) {
            putword(rel_off,ofile);
            putword(rel_seg,ofile);
        }
        rel_count++;
    }
    ohead[3]=rel_count;
    return(SUCCESS);
}

/*---------------------*/
typedef struct {
        FILE  *fp;
        WORD  buf;
        BYTE  count;
    } bitstream;

void initbits(bitstream *,FILE *);
int getbit(bitstream *);

/*---------------------*/
/* decompressor routine */
int unpack(FILE *ifile,FILE *ofile){
    int len;
    int span;
    long fpos;
    bitstream bits;
    static BYTE data[0x4500], *p=data;
    size_t minusspan;

    fpos=(long)(ihead[0x0b]-inf[4]+ihead[4])<<4;
    fseek(ifile,fpos,SEEK_SET);
    fpos=(long)ohead[4]<<4;
    if (!dotmode) fseek(ofile,fpos,SEEK_SET);
    initbits(&bits,ifile);
    printf(" unpacking. ");
    fflush(NULL);
    for(;;){
        if(ferror(ifile)) {printf("\nread error\n"); return(FAILURE); }
        if(!dotmode && ferror(ofile)) {printf("\nwrite error\n"); return(FAILURE); }
        if(p-data>0x4000){
            imagesize += 0x2000;
            outsize += 0x2000;
            if (!dotmode && fwrite(data,sizeof data[0],0x2000,ofile) != 0x2000) {
                printf("\nwrite error\n");
                return(FAILURE);
            }
            p-=0x2000;
            memcpy(data,data+0x2000,p-data);
            putchar('.');
            fflush(NULL);
        }
        if(getbit(&bits)) {
            *p++=getbyte(ifile);
            continue;
        }
        if(!getbit(&bits)) {
            len=getbit(&bits)<<1;
            len |= getbit(&bits);
            len += 2;
            span=getbyte(ifile) | 0xff00;
        } else {
            span=(BYTE)getbyte(ifile);
            len=getbyte(ifile);
            span |= ((len & ~0x07)<<5) | 0xe000;
            len = (len & 0x07)+2; 
            if (len==2) {
                len=getbyte(ifile);

                if(len==0)
                    break;    /* end mark of compreesed load module */

                if(len==1)
                    continue; /* segment change */
                else
                    len++;
            }
        }
        minusspan = -(int16_t)span;
        for( ;len>0;len--,p++){
            *p=*(p - minusspan);
        }
    }
    if(p!=data) {
        imagesize += p - data;
        outsize += p - data;
        if (!dotmode && fwrite(data,sizeof data[0],p-data,ofile) != p-data) {
            printf("\nwrite error\n");
            return(FAILURE);
        }
    }
    if (!dotmode) {
        loadsize=ftell(ofile)-fpos;
        if (loadsize != imagesize) {
            printf("\nerror: loadsize != imagesize\n");
        }
    } else {
       loadsize = imagesize;
    }
    printf("end\n");
    return(SUCCESS);
}

/* write EXE header*/
int wrhead(FILE *ofile) {
    if(ihead[6]!=0) {
        if (allocdelta != 0xFFFF) {
            if (debugging & 3)
                printf("old=%04Xh ohead[5]=%04Xh inf[5]=%04Xh inf[6]=%04Xh\n",
            	(unsigned)(ohead[5] - (inf[5] + ((inf[6]+16-1)>>4) + 9)),
            	(unsigned)ohead[5],
            	(unsigned)inf[5],
            	(unsigned)inf[6]);
            ohead[5] = ihead[5] - allocdelta;
            if (debugging & 3)
                printf("new=%04Xh\n", (unsigned)ohead[5]);
        } else {
            /* pick from https://files.shikadi.net/malv/files/unlzexe.c

v0.7 Alan Modra, amodra@sirius.ucs.adelaide.edu.au, Nov 91
    [...]
    Fixed MinBSS & MaxBSS calculation (ohead[5], ohead[6]).
    Now UNLZEXE followed by LZEXE should give the original file.
            */
            ohead[5] -= inf[5] + ((inf[6]+16-1)>>4) + 9; /* v0.7 */
            /* ohead[5] = allocsize - ((loadsize+16-1)>>4); */
        }
        if(ihead[6]!=0xffff)
            ohead[6]-=(ihead[5]-ohead[5]);
    }
    ohead[1]=(loadsize+(ohead[4]<<4)) & 0x1ff;
    ohead[2]=(loadsize+(ohead[4]<<4)+0x1ff) >>9;
    if (debugging & 9) {
        unsigned ii;
        printf("Output file executable header fields:\n");
        for (ii = 0; fields[ii]; ++ii) {
            printf("%-16s %04Xh %u\n", fields[ii], ohead[ii], ohead[ii]);
        }
    }
    if (!dotmode) {
        fseek(ofile,0L,SEEK_SET);
        if (writewords(ohead, 0x0E, ofile) != 0x0E) {
            printf("error writing output header.\n");
            return (FAILURE);
        }
    }
    return SUCCESS;
}

/*-------------------------------------------*/

/* get compress information bit by bit */
void initbits(bitstream *p,FILE *filep){
    p->fp=filep;
    p->count=0x10;
    p->buf=getword(filep);
    /* printf("%04x ",p->buf); */
}

int getbit(bitstream *p) {
    int b;
    b = p->buf & 1;
    if(--p->count == 0){
        (p->buf)=getword(p->fp);
        /* printf("%04x ",p->buf); */
        p->count= 0x10;
    }else
        p->buf >>= 1;
    
    return b;
}

// GENERATED FILE — do not edit by hand.
//
// Produced by `cargo xtask docgen` (help-system phase P3). Holds the captured
// `fm-exec` fragment transcripts keyed by their script content hash
// (`FragmentScript::content_hash`). Regenerate with `cargo xtask docgen`; CI
// verifies it is up to date with `cargo xtask docgen --check`.
//
// Included by `crate::fragment` via `include!`. Entries are sorted by hash so
// the file has a stable, reviewable git diff and supports binary-search lookup.

pub static FRAGMENTS: &[(&str, Fragment)] = &[
    (
        "000b611e651eb88859e63ca377536987ff58f06adbf31265d053870834ef08fb",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "00215015ca6e22a830be989caf4eb111c19e64273b4a950853cf5ac9249ff4ac",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "02eef640d8873732e2183ed8943ea6acdc8f0c9e45fef75cec04df8a8a273ff1",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "03bb208bc3fcb14c61df67785b142664b18630b15f02fa85dfbb31504b543ecb",
        Fragment {
            transcript: "--> cosd(45)\n\nans =\n\n0.7071\n\n--> cosd(60)\n\nans =\n\n0.5000\n",
            figure: None,
        },
    ),
    (
        "07ca1a19c719770049de884514c31d824daebeb5aa672fa0fb4be793751dd737",
        Fragment {
            transcript: "--> A = [1,2,0;4,1,-1]\n\nA =\n\n    1    2    0\n    4    1   -1\n\n--> A'\n\nans =\n\n    1    4\n    2    1\n    0   -1\n\n--> A = [1+i,2-i]\n\nA =\n\n   1 + 1i   2 - 1i\n\n--> A.'\n\nans =\n\n   1 + 1i\n   2 - 1i\n",
            figure: None,
        },
    ),
    (
        "09fd9a6c3829407e4975ab001413318ceac115935e17094faf0aef3346b15553",
        Fragment {
            transcript: "--> A = rand(5);\n--> det(A)\n\nans =\n\n-0.0011\n\n--> B = A([2,1,3,4,5],:);\n--> det(B)\n\nans =\n\n0.0011\n",
            figure: None,
        },
    ),
    (
        "0dff0ecbbbce30a8c03a64d32d5e69203b26914eb5eb7152231fa725fa147c5a",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> max(A)\n\nans =\n\n   5   3   3\n\n--> max(A,[],2)\n\nans =\n\n   5\n   3\n   3\n\n--> max([5,3,2,9])\n\nans =\n\n9\n\n--> a = int8(100*randn(4))\n\na =\n\n     17    -72   -127     -1\n    -31    -32    -41     50\n     41     72      8   -113\n     46     48    127    -50\n\n--> b = int8(100*randn(4))\n\nb =\n\n     21     77     67     31\n    -25    106    -47    100\n   -124   -104     33     20\n    -69     21     57    -56\n\n--> max(a,b)\n\nans =\n\n    21    77    67    31\n   -25   106   -41   100\n    41    72    33    20\n    46    48   127   -50\n\n--> a = randn(2)\n\na =\n\n    0.1216    2.1308\n    0.1298   -0.2004\n\n--> max(a,0)\n\nans =\n\n   0.1216   2.1308\n   0.1298        0\n",
            figure: None,
        },
    ),
    (
        "1264e0ae213da2bda8ab78ce30e724b875c023c1d4ab58d0e4f27e4acd43087b",
        Fragment {
            transcript: "--> atand(1)\n\nans =\n\n45\n",
            figure: None,
        },
    ),
    (
        "13805911266a6bc03c105d9896c5c5db8f9997a626fb002b28473c8ab50d812e",
        Fragment {
            transcript: "--> a = 2\n\na =\n\n2\n\n--> b = 1:4\n\nb =\n\n   1   2   3   4\n\n--> c = a.^b\n\nc =\n\n    2    4    8   16\n\n--> c = b.^a\n\nc =\n\n    1    4    9   16\n\n--> A = [1,2;3,2]\n\nA =\n\n   1   2\n   3   2\n\n--> B = [2,1.5;0.5,0.6]\n\nB =\n\n        2   1.5000\n   0.5000   0.6000\n\n--> C = A.^B\n\nC =\n\n        1   2.8284\n   1.7321   1.5157\n",
            figure: None,
        },
    ),
    (
        "13a9db77f8e71e88efa2ac84efbe486a0c8e00c992f06cdb2d3beb0b4096316a",
        Fragment {
            transcript: "--> 3 ./ 8\n\nans =\n\n0.3750\n\n--> a = 3 + 4*i\n\na =\n\n3 + 4i\n\n--> b = 5 + 8*i\n\nb =\n\n5 + 8i\n\n--> c = a ./ b\n\nc =\n\n0.5281 - 0.0449i\n\n--> a = [1,2;3,4]\n\na =\n\n   1   2\n   3   4\n\n--> b = [2,3;6,7]\n\nb =\n\n   2   3\n   6   7\n\n--> c = a ./ b\n\nc =\n\n   0.5000   0.6667\n   0.5000   0.5714\n\n--> c = a ./ 3\n\nc =\n\n   0.3333   0.6667\n        1   1.3333\n\n--> c = 3 ./ a\n\nc =\n\n        3   1.5000\n        1   0.7500\n",
            figure: None,
        },
    ),
    (
        "1507adca69234d89782e235a41ab13270fba5ae0b3cfbddee736c19a53ad6100",
        Fragment {
            transcript: "--> zeros(2,3,2)\n\nans =\n\n(:,:,1) =\n\n   0   0   0\n   0   0   0\n\n(:,:,2) =\n\n   0   0   0\n   0   0   0\n\n--> zeros(1,3)\n\nans =\n\n   0   0   0\n\n--> zeros([2,6])\n\nans =\n\n   0   0   0   0   0   0\n   0   0   0   0   0   0\n\n--> zeros([1,3])\n\nans =\n\n   0   0   0\n\n--> uint16(zeros(3))\n\nans =\n\n   0   0   0\n   0   0   0\n   0   0   0\n\n--> zeros(3,'int16')\n\nans =\n\n   0   0   0\n   0   0   0\n   0   0   0\n",
            figure: None,
        },
    ),
    (
        "16155b4788c5c8fd254a120768216961e53064bac8fca6fb6673358e6dcfc872",
        Fragment {
            transcript: "--> mod(18,12)\n\nans =\n\n6\n\n--> mod(6,5)\n\nans =\n\n1\n\n--> mod(2*pi,pi)\n\nans =\n\n0\n\n--> mod([1,3,5,2],2)\n\nans =\n\n0\n\n--> mod([9 3 2 0],[1 0 2 2])\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "193e3b72e2862a513ec26322d5ebaa5705abdd633c07601e02eae5cce883a9bc",
        Fragment {
            transcript: "--> a = uint8(1:6)\n\na =\n\n   1   2   3   4   5   6\n\n--> reshape(a,2,3)\n\nans =\n\n   1   3   5\n   2   4   6\n\n--> a = uint8(1:12)\n\na =\n\n    1    2    3    4    5    6    7    8    9   10   11   12\n\n--> reshape(a,[2,3,2])\n\nans =\n\n(:,:,1) =\n\n    1    3    5\n    2    4    6\n\n(:,:,2) =\n\n    7    9   11\n    8   10   12\n\n--> a = [1,6,7;3,4,2]\n\na =\n\n   1   6   7\n   3   4   2\n\n--> reshape(a,3,2)\n\nans =\n\n   1   4\n   3   7\n   6   2\n",
            figure: None,
        },
    ),
    (
        "19e736706ec927f0d39d00a069afa2da2c651e9e6e9dd0e77cc39d99f25a857e",
        Fragment {
            transcript: "--> 3 .* 8\n\nans =\n\n24\n\n--> 3.1 .* [2,4,5,6,7]\n\nans =\n\n    6.2000   12.4000   15.5000   18.6000   21.7000\n\n--> a = 3 + 4*i\n\na =\n\n3 + 4i\n\n--> b = a .* 2\n\nb =\n\n6 + 8i\n\n--> a = [1,2;3,4]\n\na =\n\n   1   2\n   3   4\n\n--> b = [2,3;6,7]\n\nb =\n\n   2   3\n   6   7\n\n--> c = a .* b\n\nc =\n\n    2    6\n   18   28\n",
            figure: None,
        },
    ),
    (
        "1dded79d7ace0b550f0a209c18b5d593eba822b9b524d24dbd8ab3629dff0559",
        Fragment {
            transcript: "--> nan*0\n\nans =\n\nNaN\n\n--> nan-nan\n\nans =\n\nNaN\n\n--> uint32(nan)\n\nans =\n\n0\n\n--> complex(nan)\n\nans =\n\nNaN + 0i\n",
            figure: None,
        },
    ),
    (
        "1eb1fc89215dd54b7dd5db8dbe899d999d51925bc7effbb6b0a2eb8eff380c3b",
        Fragment {
            transcript: "--> fp = fopen('test.dat','w','ieee-le')\n\nfp =\n\n8\n\n--> fwrite(fp,float([1.2,4.3,2.1]))\n\nans =\n\n3\n\n--> fclose(fp)\n\nans =\n\n0\n\n--> fp = fopen('test.dat','r','ieee-le')\n\nfp =\n\n9\n\n--> fread(fp,[1,3],'float')\n\nans =\n\n   1.2000\n   4.3000\n   2.1000\n\n--> fclose(fp)\n\nans =\n\n0\n\n--> fp = fopen('test.dat','a+','le')\n\nfp =\n\n10\n\n--> fwrite(fp,float([pi,e]))\n\nans =\n\n2\n\n--> fclose(fp)\n\nans =\n\n0\n\n--> fp = fopen('test.dat','r','ieee-le')\n\nfp =\n\n11\n\n--> fread(fp,[1,5],'float')\n\nans =\n\n   1.2000\n   4.3000\n   2.1000\n   3.1416\n   2.7183\n\n--> fclose(fp)\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "204712f05ec376253fc8ef2ac19a9dbd17b0cdc2d4e9859615a0d9a69f14ea7d",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> sum(A)\n\nans =\n\n   8   6   5\n\n--> sum(A,2)\n\nans =\n\n   9\n   6\n   4\n",
            figure: None,
        },
    ),
    (
        "20cbb5c67d7f710032b1bf7522e8626390a724264d7906f73ee8958fcab39a77",
        Fragment {
            transcript: "--> [start,stop,tokenExtents,match,tokens,named] = regexp('quick down town zoo','(.)own')\n\nstart =\n\n    7   12\n\n\nstop =\n\n   10   15\n\n\ntokenExtents =\n\n{\n  ['down']  ['town']\n}\n\n\nmatch =\n\n{\n  [1x1 cell]  [1x1 cell]\n}\n\n\ntokens =\n\n{\n  ['quick ']  [' ']  [' zoo']\n}\n",
            figure: None,
        },
    ),
    (
        "211abe7998d7f554b736e9af04358494a175f236a4b4cde6225b4183b6a011a7",
        Fragment {
            transcript: "--> num2hex([1 0 0.1 -pi inf nan])\n\nans =\n\n   3   f   f   0   0   0   0   0   0   0   0   0   0   0   0   0\n   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0\n   3   f   b   9   9   9   9   9   9   9   9   9   9   9   9   a\n   c   0   0   9   2   1   f   b   5   4   4   4   2   d   1   8\n   7   f   f   0   0   0   0   0   0   0   0   0   0   0   0   0\n   7   f   f   8   0   0   0   0   0   0   0   0   0   0   0   0\n\n--> num2hex(float([1 0 0.1 -pi inf nan]))\n\nans =\n\n   3   f   f   0   0   0   0   0   0   0   0   0   0   0   0   0\n   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0\n   3   f   b   9   9   9   9   9   a   0   0   0   0   0   0   0\n   c   0   0   9   2   1   f   b   6   0   0   0   0   0   0   0\n   7   f   f   0   0   0   0   0   0   0   0   0   0   0   0   0\n   7   f   f   8   0   0   0   0   0   0   0   0   0   0   0   0\n",
            figure: None,
        },
    ),
    (
        "217f8a83a54bdf144754f1c01f978891a6ed3ea4f67c226f6706d01591a4e2d7",
        Fragment {
            transcript: "--> logical([1,2,3,0,0,0,5,2,2])\n\nans =\n\n   1   1   1   0   0   0   1   1   1\n\n--> logical([pi,pi,0,e,0,-1])\n\nans =\n\n   1   1   0   1   0   1\n",
            figure: None,
        },
    ),
    (
        "21e7df5313d10394f1050b5807bc331c85a8a3d44ddeae051f8635ce37c9eb8b",
        Fragment {
            transcript: "--> a = [1,0,4,2,0;0,0,0,0,0;0,1,0,0,2]\n\na =\n\n   1   0   4   2   0\n   0   0   0   0   0\n   0   1   0   0   2\n\n--> A = sparse(a)\n\nA =\n\n  (1,1)      1\n  (3,2)      1\n  (1,3)      4\n  (1,4)      2\n  (3,5)      2\n\n--> full(A)\n\nans =\n\n   1   0   4   2   0\n   0   0   0   0   0\n   0   1   0   0   2\n",
            figure: None,
        },
    ),
    (
        "231ed1c7370b9ca4d4d2107f7255b2ba60e01f08dcefa82f254bd0a74be5cfaa",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "23277f7f49346b92c974a305504c065c379b4f09369082841c6d6fce3d38cd23",
        Fragment {
            transcript: "--> A = [2;5;6;2]\n\nA =\n\n   2\n   5\n   6\n   2\n\n--> int2bin(A,8)\n\nans =\n\n   0   0   0   0   0   0   1   0\n   0   0   0   0   0   1   0   1\n   0   0   0   0   0   1   1   0\n   0   0   0   0   0   0   1   0\n\n--> A = [1;2;-5;2]\n\nA =\n\n    1\n    2\n   -5\n    2\n\n--> int2bin(A,8)\n\nans =\n\n   0   0   0   0   0   0   0   1\n   0   0   0   0   0   0   1   0\n   1   1   1   1   1   0   1   1\n   0   0   0   0   0   0   1   0\n",
            figure: None,
        },
    ),
    (
        "25ed48afd1fc73a1a7d35bfcff528480c5ddf47f838bbe005d2c8b6a76b96b6a",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> cumsum(A)\n\nans =\n\n   5   1   3\n   8   3   4\n   8   6   5\n\n--> cumsum(A,2)\n\nans =\n\n   5   6   9\n   3   5   6\n   0   3   4\n\n--> B(:,:,1) = [5,2;8,9];\n--> B(:,:,2) = [1,0;3,0]\n\nB =\n\n(:,:,1) =\n\n   5   2\n   8   9\n\n(:,:,2) =\n\n   1   0\n   3   0\n\n--> cumsum(B,3)\n\nans =\n\n(:,:,1) =\n\n    5    2\n    8    9\n\n(:,:,2) =\n\n    6    2\n   11    9\n",
            figure: None,
        },
    ),
    (
        "2634eecaf44ade3514c741b8e5c7756084f4975da143725cd8e7fd173ef4dc88",
        Fragment {
            transcript: "--> A = {'Hello','Yellow';'Mellow','Othello'}\n\nA =\n\n{\n  ['Hello']  ['Yellow']\n  ['Mellow']  ['Othello']\n}\n\n--> iscellstr(A)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "263f149c05167a391748463c99df503384feef08f05dea75039835192607d937",
        Fragment {
            transcript: "--> x = rand(4,4,3);\n--> length(x)\n\nans =\n\n4\n",
            figure: None,
        },
    ),
    (
        "2a1ea581002e85183380fb0157640ef33c902e83818879fe6654d42f52778c33",
        Fragment {
            transcript: "--> B = [1,1;0,1];\n--> A = [4,5]\n\nA =\n\n   4   5\n\n--> A/B\n\nans =\n\n   4   1\n",
            figure: None,
        },
    ),
    (
        "2a92326f1615174210ae5f5c8ca035d16f219f8f48f2b6c4c93655ebbbd7117f",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "2c008f8b6b63ad12676b050743b22e6b79b3597379abf168277855591142cdcb",
        Fragment {
            transcript: "--> for i=1:10; count_calls; end\nFunction has been called 1 times\nFunction has been called 2 times\nFunction has been called 3 times\nFunction has been called 4 times\nFunction has been called 5 times\nFunction has been called 6 times\nFunction has been called 7 times\nFunction has been called 8 times\nFunction has been called 9 times\nFunction has been called 10 times\n",
            figure: None,
        },
    ),
    (
        "2f5b02af7e10a1b971789860d1a47cbe79a810e8d9352bad1b277a26864dd953",
        Fragment {
            transcript: "--> acosd(sqrt(2)/2)\n\nans =\n\n45\n\n--> acosd(0.5)\n\nans =\n\n60.0000\n",
            figure: None,
        },
    ),
    (
        "3581780205700bc1524085d02c2c80518d961a416729fc1282402175165e1a37",
        Fragment {
            transcript: "--> A = float(randn(512));\n--> fp = fopen('test.dat','w');\n--> fwrite(fp,A,'single');\n--> fclose(fp);\n",
            figure: None,
        },
    ),
    (
        "3762ca8aa1279e8073dbf176dbe9145b7ff34f7d97fca22216ae83110a644455",
        Fragment {
            transcript: "--> eps\n\nans =\n\n2.2204e-16\n\n--> 1.0+eps\n\nans =\n\n1.0000\n\n--> eps(1000.)\n\nans =\n\n1.1369e-13\n",
            figure: None,
        },
    ),
    (
        "3960ec08d0be3b5b9390d57b3da40aaab08937b319f34b17098bcea391101c7c",
        Fragment {
            transcript: "--> accum = 0;\n--> k=1;\n--> while (k<=100), accum = accum + k; k = k + 1; end\n--> accum\n\nans =\n\n5050\n",
            figure: None,
        },
    ),
    (
        "3b0aee514e0dc9e029b12d941fa810d46f29c75de65f7f862f79da7e47b258ea",
        Fragment {
            transcript: "--> if_test(1)\n\nans =\n\none\n\n--> if_test(2)\n\nans =\n\ntwo\n\n--> if_test(3)\n\nans =\n\nthree\n\n--> if_test(pi)\n\nans =\n\nsomething else\n",
            figure: None,
        },
    ),
    (
        "3ec5fa162f719308c47d23a5526164fc517b4a5a094d90ed00f98c3de9a9d6fb",
        Fragment {
            transcript: "--> accum = 0;\n--> for (i=1:100); accum = accum + i; end\n--> accum\n\nans =\n\n5050\n\n--> accum = 0;\n--> for i=1:100; accum = accum + i; end\n--> accum\n\nans =\n\n5050\n",
            figure: None,
        },
    ),
    (
        "41b903a81242d389e517335f7dad6d3194b4f0bd3b6aea648bfe60955f8cefa3",
        Fragment {
            transcript: "--> A = [1,2;5,8]\n\nA =\n\n   1   2\n   5   8\n\n--> B = [A,[3.2f;5.1f]]\n\nB =\n\n        1        2   3.2000\n        5        8   5.1000\n\n--> C = [B;5.2,1.0,0.0]\n\nC =\n\n        1        2   3.2000\n        5        8   5.1000\n   5.2000        1        0\n\n--> D = [B;2.0f+3.0f*i,i,0.0f]\n\nD =\n\n        1 + 0i        2 + 0i   3.2000 + 0i\n        5 + 0i        8 + 0i   5.1000 + 0i\n        2 + 3i        0 + 1i        0 + 0i\n\n--> E = [B;2.0+3.0*i,i,0.0]\n\nE =\n\n        1 + 0i        2 + 0i   3.2000 + 0i\n        5 + 0i        8 + 0i   5.1000 + 0i\n        2 + 3i        0 + 1i        0 + 0i\n\n--> F = ['hello';'there']\n\nF =\n\n   h   e   l   l   o\n   t   h   e   r   e\n",
            figure: None,
        },
    ),
    (
        "428bd5558dad9b632cbb5036c3a58cd388fcd902096cf9c7730198b8bf294fcc",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> var(A)\n\nans =\n\n   6.3333        1   1.3333\n\n--> var(A,2)\n\nans =\n\n        4\n        1\n   2.3333\n",
            figure: None,
        },
    ),
    (
        "4347d6ccd726fdafb107d873fb35c55c637d84d2899b1497a138cb141e3745a7",
        Fragment {
            transcript: "--> dec2bin(56)\n\nans =\n\n111000\n\n--> dec2bin(1039456)\n\nans =\n\n11111101110001100000\n\n--> dec2bin([63,73,32],5)\n\nans =\n\n   0   1   1   1   1   1   1\n   1   0   0   1   0   0   1\n   0   1   0   0   0   0   0\n",
            figure: None,
        },
    ),
    (
        "453d11334ca92e901158d430d809117eb3e95b2f86e9048b4fed8be6980a6093",
        Fragment {
            transcript: "--> eye(3)\n\nans =\n\n   1   0   0\n   0   1   0\n   0   0   1\n",
            figure: None,
        },
    ),
    (
        "45e48220fc5ffa19205524d59de3e32f33073278a7495769f4803bf945d48fb4",
        Fragment {
            transcript: "--> acscd(2/sqrt(2))\n\nans =\n\n45.0000\n\n--> acscd(0.5)\n\nans =\n\nNaN\n",
            figure: None,
        },
    ),
    (
        "46371684b58768b50095288d5b906120cecb1534d7ef1419dbc079167ac4fa0e",
        Fragment {
            transcript: "--> wrapcall('printf','%f...%f\\n',pi,e)\n3.141593...2.718282\n",
            figure: None,
        },
    ),
    (
        "46a0ea54287f93a3ccb91e803be0c334846d5097573d03299ec85bfbc43707ce",
        Fragment {
            transcript: "--> A = randi(ones(3,4),10*ones(3,4))\n\nA =\n\n   7   8   1   4\n   5   3   7   6\n   7   4   9   1\n\n--> [d1 d2] = ind2sub(size(A),7)\n\nd1 =\n\n1\n\n\nd2 =\n\n3\n\n--> A(d1,d2)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "4790703f5269770b800ab24a20bfecc059f0b69794c2fc0759de307dfe00b987",
        Fragment {
            transcript: "--> A = float(randn(2,3))\n\nA =\n\n    0.1710    0.4110   -0.7223\n   -0.3091    0.4638   -0.3199\n\n--> [U,S,V] = svd(A)\n\nU =\n\n    0.8319   -0.5549\n    0.5549    0.8319\n\n\nS =\n\n   0.9828        0        0\n        0   0.4087        0\n\n\nV =\n\n   -0.0298   -0.8616    0.5068\n    0.6098    0.3861    0.6922\n   -0.7920    0.3296    0.5139\n\n--> U*S*V'\n\nans =\n\n    0.1710    0.4110   -0.7223\n   -0.3091    0.4638   -0.3199\n\n--> svd(A)\n\nans =\n\n   0.9828\n   0.4087\n",
            figure: None,
        },
    ),
    (
        "48143052cd7104bef62e2a11f7c3767d51beb56b05a9765300b42943eb2198af",
        Fragment {
            transcript: "--> A = randi(ones(3,4),10*ones(3,4))\n\nA =\n\n   7   8   1   4\n   5   3   7   6\n   7   4   9   1\n\n--> n = sub2ind(size(A),1:3,2:4)\n\nn =\n\n    4    8   12\n\n--> A(n)\n\nans =\n\n   8   7   1\n",
            figure: None,
        },
    ),
    (
        "49651d68eb8cf0f7ff706cd470a10770e1fe35bbe1de7a3b70daaf7e6e50d2a4",
        Fragment {
            transcript: "--> polyint([2,3,4])\n\nans =\n\n   0.6667   1.5000        4        0\n\n--> polyint([2,3,4],5)\n\nans =\n\n   0.6667   1.5000        4        5\n",
            figure: None,
        },
    ),
    (
        "4a2373e603a3e2874c12ce96fa6767a7b2de856c0fd373dd962a39749070a3ff",
        Fragment {
            transcript: "--> polyder([2,3,4])\n\nans =\n\n   4   3\n\n--> polyder([2,3,4],7)\n\nans =\n\n   4   3\n\n--> [n,d] = polyder([2,3,4],5)\n\nn =\n\n   4   3\n",
            figure: None,
        },
    ),
    (
        "4adf667a3b829af6f76092e82de60a6716b8250821fdd31ef44a376b9caa6ce0",
        Fragment {
            transcript: "--> imag(3+4*i)\n\nans =\n\n4\n\n--> imag([2,4,5,6])\n\nans =\n\n   0   0   0   0\n\n--> imag([2.0+3.0*i,i])\n\nans =\n\n   3   1\n",
            figure: None,
        },
    ),
    (
        "4c1988c2077f89e888407c8458fdfd3c9b1d68ee66ca3b7486209fcf0ad16321",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> std(A)\n\nans =\n\n   2.5166        1   1.1547\n\n--> std(A,2)\n\nans =\n\n        2\n        1\n   1.5275\n",
            figure: None,
        },
    ),
    (
        "4d3b45dd45e93bb5ff5509c3fa55378500e54f460ba3356a567540f7f7cfea88",
        Fragment {
            transcript: "--> fp = fopen('testtext','w');\n--> fprintf(fp,'String 1\\n');\n--> fprintf(fp,'String 2\\n');\n--> fclose(fp);\n--> fp = fopen('testtext','r')\n\nfp =\n\n7\n\n--> fgetline(fp)\n\nans =\n\nString 1\n\n--> fgetline(fp)\n\nans =\n\nString 2\n\n--> fclose(fp);\n",
            figure: None,
        },
    ),
    (
        "4f04e34701d5ca995a331feaac64e893025e157a3a3be66839a36fd69eda5f25",
        Fragment {
            transcript: "--> printf('intvalue is %d, floatvalue is %f\\n',3,1.53);\nintvalue is 3, floatvalue is 1.530000\n--> printf('string value is %s\\n','hello');\nstring value is hello\n--> printf('integer padded is %012d\\n',32);\ninteger padded is 000000000032\n--> printf('float value is %+018.12f\\n',pi);\nfloat value is 00003.141592653590\n",
            figure: None,
        },
    ),
    (
        "4f502435b0077e8636598b5e8c10119dfc6ae1b4646066f1dbb2e350e0614be0",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> mean(A)\n\nans =\n\n   2.6667        2   1.6667\n\n--> mean(A,2)\n\nans =\n\n        3\n        2\n   1.3333\n",
            figure: None,
        },
    ),
    (
        "515a408755f202510520d8b019a14589ecf06b58b84fbd74ade89ebb24b89f52",
        Fragment {
            transcript: "--> which fft\n--> which fliplr\n",
            figure: None,
        },
    ),
    (
        "52473231bc7c8514733a303021532d565d08674dc67cd1cc6a40fa3f361a5e78",
        Fragment {
            transcript: "--> dec2hex(1023)\n\nans =\n\n3FF\n\n--> dec2hex(58128493)\n\nans =\n\n376F86D\n",
            figure: None,
        },
    ),
    (
        "52563adeeb76e8f8aa85b7082b244b5614f4e620ca581fcc1b4ae58b98e967b1",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> cumprod(A)\n\nans =\n\n    5    1    3\n   15    2    3\n    0    6    3\n\n--> cumprod(A,2)\n\nans =\n\n    5    5   15\n    3    6    6\n    0    0    0\n\n--> B(:,:,1) = [5,2;8,9];\n--> B(:,:,2) = [1,0;3,0]\n\nB =\n\n(:,:,1) =\n\n   5   2\n   8   9\n\n(:,:,2) =\n\n   1   0\n   3   0\n\n--> cumprod(B,3)\n\nans =\n\n(:,:,1) =\n\n    5    2\n    8    9\n\n(:,:,2) =\n\n    5    0\n   24    0\n",
            figure: None,
        },
    ),
    (
        "534e1ec9f1e5deb5d8b87a8c8b5f8cec21cae01c4e460050bdd218e55876aacc",
        Fragment {
            transcript: "--> bitand(uint16([1,16,255]),uint16([3,17,128]))\n\nans =\n\n     1    16   128\n\n--> bitand(uint16([1,16,255]),uint16(3))\n\nans =\n\n   1   0   3\n",
            figure: None,
        },
    ),
    (
        "544b02007cac8aee91f031f1707ab1c4d822e473eb61c4e84ca60df87db25dc2",
        Fragment {
            transcript: "--> fp = fopen('test.dat','wb','ieee-le')\n\nfp =\n\n5\n\n--> fclose(fp)\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "5649ea420232b4f5366b3f04d468338499dae397d24b9ff9555e289f99ef23f9",
        Fragment {
            transcript: "--> A = int32(10*rand(4,5))\n\nA =\n\n   6   2   9   5   2\n   4   3   4   6   4\n   7   0   5   1   6\n   8   7   1   0   3\n\n--> diag(A)\n\nans =\n\n   6\n   3\n   5\n   0\n\n--> diag(A,1)\n\nans =\n\n   2\n   4\n   1\n   3\n\n--> x = int32(10*rand(1,3))\n\nx =\n\n   0   1   7\n\n--> diag(x)\n\nans =\n\n   0   0   0\n   0   1   0\n   0   0   7\n\n--> diag(x,-1)\n\nans =\n\n   0   0   0   0\n   0   0   0   0\n   0   1   0   0\n   0   0   7   0\n",
            figure: None,
        },
    ),
    (
        "5c689669d1892e0d7c5780889b32b9c0f2c24053a5f0f359315e7c5477106664",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "5c8197301bc8df25f8651151fb58c675e6acbb6463122740079d6fa44a7b1a05",
        Fragment {
            transcript: "--> A = [1,0,0;1,0,0;0,0,1]\n\nA =\n\n   1   0   0\n   1   0   0\n   0   0   1\n\n--> any(A)\n\nans =\n\n   1   0   1\n\n--> any(A,2)\n\nans =\n\n   1\n   1\n   1\n",
            figure: None,
        },
    ),
    (
        "5d7bee3ff9f03f6a6f4f7b9386c8768abdc8c0061539843fa0bca2a614ad3eec",
        Fragment {
            transcript: "--> sind(45)\n\nans =\n\n0.7071\n\n--> sind(30)\n\nans =\n\n0.5000\n",
            figure: None,
        },
    ),
    (
        "5df8fa2ffcad99c9373cb1f5bd0527fe23b929c407da91f9daaef173319176fc",
        Fragment {
            transcript: "--> x1 = 'astring';\n--> x2 = 'bstring';\n--> x3 = 'astring';\n--> strcmp(x1,x2)\n\nans =\n\n0\n\n--> strcmp(x1,x3)\n\nans =\n\n1\n\n--> x = {'astring','bstring',43,'astring'}\n\nx =\n\n{\n  ['astring']  ['bstring']  [43]  ['astring']\n}\n\n--> p = strcmp(x,'astring')\n\np =\n\n0\n\n--> strcmp({'this','is','a','pickle'},{'what','is','to','pickle'})\n\nans =\n\n0\n\n--> strcmp({'this','is','a','pickle'},['peter ';'piper ';'hated ';'pickle'])\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "5e5bf3c8300fa1131dbe640377a8ab9aac984ea4883e5a1f285d5cdbe62f934a",
        Fragment {
            transcript: "--> x = linspace(0,1,5)\n\nx =\n\n        0   0.2500   0.5000   0.7500        1\n",
            figure: None,
        },
    ),
    (
        "5ee70fd3a13d6d01b83fa9747e06268ddb48dad2c59930234e9a8ccbd5e53b8f",
        Fragment {
            transcript: "--> 3 .\\ 8\n\nans =\n\n2.6667\n\n--> a = 3 + 4*i\n\na =\n\n3 + 4i\n\n--> b = 5 + 8*i\n\nb =\n\n5 + 8i\n\n--> c = b .\\ a\n\nc =\n\n0.5281 - 0.0449i\n\n--> a = [1,2;3,4]\n\na =\n\n   1   2\n   3   4\n\n--> b = [2,3;6,7]\n\nb =\n\n   2   3\n   6   7\n\n--> c = a .\\ b\n\nc =\n\n        2   1.5000\n        2   1.7500\n\n--> c = a .\\ 3\n\nc =\n\n        3   1.5000\n        1   0.7500\n\n--> c = 3 .\\ a\n\nc =\n\n   0.3333   0.6667\n        1   1.3333\n",
            figure: None,
        },
    ),
    (
        "5f8d2118d48ed312d487c40772f85ab682b565126ac49602e2a1b2c5f8b52938",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "5f9c1ee879f233e9026b29adddbcbb83a1dfbd04a825e05e635faa8fb189213d",
        Fragment {
            transcript: "--> iscell('foo')\n\nans =\n\n0\n\n--> iscell(2)\n\nans =\n\n0\n\n--> iscell({1,2,3})\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "625faaad5a295c5ea9950246f85c5aea928ceea87c8a7bbe9ea248066aabc4a0",
        Fragment {
            transcript: "--> clear a b\n--> source source_test\na is 32 and b is 32\n",
            figure: None,
        },
    ),
    (
        "64bf569477878efd8cbff45c5eaea15117ac2425a87d04e2f895a9f25582b604",
        Fragment {
            transcript: "--> seed(32,41);\n--> rand(1,5)\n\nans =\n\n   0.5831   0.3463   0.7738   0.5535   0.4408\n\n--> seed(32,41);\n--> rand(1,5)\n\nans =\n\n   0.5831   0.3463   0.7738   0.5535   0.4408\n",
            figure: None,
        },
    ),
    (
        "666deffe46fd4dcef843ea0d2e8c15cc55be66cf296880bb33c012583719f537",
        Fragment {
            transcript: "--> a = 53\n\na =\n\n53\n\n--> clear a\n--> a\n  \u{1b}[31m×\u{1b}[0m undefined variable or function 'a'\n   ╭────\n \u{1b}[2m1\u{1b}[0m │ a\n   · \u{1b}[35;1m┬\u{1b}[0m\n   · \u{1b}[35;1m╰── \u{1b}[35;1mhere\u{1b}[0m\u{1b}[0m\n   ╰────\n",
            figure: None,
        },
    ),
    (
        "68c94aaabffe38a9d699f15b733666a5d9db502e70725442c31c9615b976dd2c",
        Fragment {
            transcript: "--> randi(zeros(1,6),5*ones(1,6))\n\nans =\n\n   3   2   4   4   1   1\n",
            figure: None,
        },
    ),
    (
        "6985bf3b1ee13744cc44315febaba2650324b7d8a9f0d5437b689587e9276896",
        Fragment {
            transcript: "--> a = randi(zeros(3),5*ones(3))\n\na =\n\n   3   4   0\n   2   1   3\n   4   1   5\n\n--> b = inv(a)\n\nb =\n\n    0.1429   -1.4286    0.8571\n    0.1429    1.0714   -0.6429\n   -0.1429    0.9286   -0.3571\n\n--> a*b\n\nans =\n\n             1    8.8818e-16             0\n   -5.5511e-17             1    1.1102e-16\n   -5.5511e-17    1.1102e-16             1\n\n--> b*a\n\nans =\n\n            1            0   7.7716e-16\n            0            1   4.4409e-16\n            0            0            1\n",
            figure: None,
        },
    ),
    (
        "69ca107a0068ea1ef02ab95cbaca64edc554390e52c2ece8ad6b030b095189da",
        Fragment {
            transcript: "--> A = rand(30);\n--> rcond(A)\n\nans =\n\n0.0013\n\n--> 1/(norm(A,1)*norm(inv(A),1))\n\nans =\n\n0.0013\n",
            figure: None,
        },
    ),
    (
        "6a9020c05fcd77e9e881158b0e520e91658ed6510ea38a232a3412d8eb26a44f",
        Fragment {
            transcript: "--> switch_test('root beer')\n\nans =\n\nfood\n\n--> switch_test('red')\n\nans =\n\ncolor\n\n--> switch_test('carpet')\n\nans =\n\nnot sure\n",
            figure: None,
        },
    ),
    (
        "6c8efe79096be0cade3e311a9b78a8d67709e26ef30386f63a02c528041706bc",
        Fragment {
            transcript: "--> [a,b] = ndgrid(1:2,3:5)\n\na =\n\n   1   1   1\n   2   2   2\n\n\nb =\n\n   3   4   5\n   3   4   5\n\n--> [a,b,c] = ndgrid(1:2,3:5,0:1)\n\na =\n\n   1   1   1\n   2   2   2\n\n\nb =\n\n   3   4   5\n   3   4   5\n\n--> [a,b,c] = ndgrid(1:3)\n\na =\n\n   1   1   1\n   2   2   2\n   3   3   3\n\n\nb =\n\n   1   2   3\n   1   2   3\n   1   2   3\n",
            figure: None,
        },
    ),
    (
        "6ea0dd613ef38989bcb188f8ae1fff79a4eba7d4be5384a8023c92e342502228",
        Fragment {
            transcript: "--> read_file('this_filename_is_invalid')\n\nc =\n\ncould not open file because of error :fgetl: invalid file id\n\n\nans =\n\ncould not open file because of error :fgetl: invalid file id\n\n--> fp = fopen('test_text.txt','w');\n--> fprintf(fp,'a line of text\\n');\n--> fclose(fp);\n--> read_file('test_text.txt')\n\nans =\n\na line of text\n",
            figure: None,
        },
    ),
    (
        "6f42a89a2f18c9f6a857a2bd1f76e3adda89573a7a0f5ba0dfaf9cd3adb3c192",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> prod(A)\n\nans =\n\n   0   6   3\n\n--> prod(A,2)\n\nans =\n\n   15\n    6\n    0\n",
            figure: None,
        },
    ),
    (
        "703aa19e15bb03b2445eb15bad98e741fa3938a21dd0184d361ec423fbe8b93d",
        Fragment {
            transcript: "--> a = {1}\n\na =\n\n{\n  [1]\n}\n\n--> isa(a,'char')\n\nans =\n\n0\n\n--> isa(a,'cell')\n\nans =\n\n1\n\n--> a = 'hello'\n\na =\n\nhello\n\n--> isa(a,'char') && strcmp(a,'hello')\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "705b8ab08cfdc18beb678d68e9bd53e9f140d70ec3eb0bc6c1155ba41f91895f",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "738b2cd48a154d62a924dc379630a0f5135b639cb8a52c03282271d1e623f37d",
        Fragment {
            transcript: "--> x = zeros(1,4,3,1,1,2);\n--> size(x)\n\nans =\n\n   1   4   3   1   1   2\n\n--> y = squeeze(x);\n--> size(y)\n\nans =\n\n   4   3   2\n",
            figure: None,
        },
    ),
    (
        "7635d963b661f8cb292d7341ad55b0642906da937b5c591568a902818cde6faa",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "7788d02bbc2eefd8e36719d10b3737f598d9fb79581a083e65274c86e0ff7f42",
        Fragment {
            transcript: "--> a = 'how now brown cow?'\n\na =\n\nhow now brown cow?\n\n--> b = strfind(a,'ow')\n\nb =\n\n    2    6   11   16\n\n--> a = {'how now brown cow','quick brown fox','coffee anyone?'}\n\na =\n\n{\n  ['how now brown cow']  ['quick brown fox']  ['coffee anyone?']\n}\n\n--> b = strfind(a,'ow')\n\nb =\n\n[]\n",
            figure: None,
        },
    ),
    (
        "79db0097bcab69408f9a8ca21e23d4084f34e0c2513828e29314ef04522c9dae",
        Fragment {
            transcript: "--> rem(18,12)\n\nans =\n\n6\n\n--> rem(6,5)\n\nans =\n\n1\n\n--> rem(2*pi,pi)\n\nans =\n\n0\n\n--> rem([1,3,5,2],2)\n\nans =\n\n0\n\n--> rem([9 3 2 0],[1 0 2 2])\n\nans =\n\nNaN\n",
            figure: None,
        },
    ),
    (
        "7b90a2b9b203b2827039df6d9ffe117c8ae13a04d8c4894993e430f595e6f4db",
        Fragment {
            transcript: "--> A = [1,1;0,1]\n\nA =\n\n   1   1\n   0   1\n\n--> B = [3;2]\n\nB =\n\n   3\n   2\n\n--> Y = A\\B\n\nY =\n\n   1\n   2\n\n--> A = [1;1]\n\nA =\n\n   1\n   1\n\n--> B = [2;1]\n\nB =\n\n   2\n   1\n\n--> A\\B\n\nans =\n\n1.5000\n",
            figure: None,
        },
    ),
    (
        "7b9a9baa87e40194638d4cfa344ae840e1045e4c767798994d5825a3db827d19",
        Fragment {
            transcript: "--> typeof({1})\n\nans =\n\ncell\n\n--> typeof(struct('foo',3))\n\nans =\n\nstruct\n\n--> typeof(3>5)\n\nans =\n\nlogical\n\n--> typeof(uint8(3))\n\nans =\n\nuint8\n\n--> typeof(int8(8))\n\nans =\n\nint8\n\n--> typeof(uint16(3))\n\nans =\n\nuint16\n\n--> typeof(int16(8))\n\nans =\n\nint16\n\n--> typeof(uint32(3))\n\nans =\n\nuint32\n\n--> typeof(int32(3))\n\nans =\n\nint32\n\n--> typeof(uint64(3))\n\nans =\n\nuint64\n\n--> typeof(int64(3))\n\nans =\n\nint64\n\n--> typeof(1.0f)\n\nans =\n\nsingle\n\n--> typeof(1.0D)\n\nans =\n\ndouble\n\n--> typeof(1.0f+i)\n\nans =\n\nsingle\n\n--> typeof(1.0D+2.0D*i)\n\nans =\n\ndouble\n",
            figure: None,
        },
    ),
    (
        "7be98c4bf7c1f85e0e464abc761b381f179f46fe5ff9bb477e422f38421ad0a1",
        Fragment {
            transcript: "--> x1 = 'astring';\n--> x2 = 'bstring';\n--> x3 = 'astring';\n--> strncmp(x1,x2,4)\n\nans =\n\n0\n\n--> strncmp(x1,x3,4)\n\nans =\n\n1\n\n--> x = {'ast','bst',43,'astr'}\n\nx =\n\n{\n  ['ast']  ['bst']  [43]  ['astr']\n}\n\n--> p = strncmp(x,'ast',3)\n\np =\n\n0\n\n--> strncmp({'this','is','a','pickle'},{'think','is','to','pickle'},3)\n\nans =\n\n0\n\n--> strncmp({'this','is','a','pickle'},['peter ';'piper ';'hated ';'pickle'],4);\n",
            figure: None,
        },
    ),
    (
        "7e84ca55d2307c3d15ee8d3dd5778f86c207c903387daf41cd9218f8d4b4a4ac",
        Fragment {
            transcript: "--> l = {}; for i = 1:5; s = sprintf('file_%d.dat',i); l(i) = {s}; end;\n--> l\n\nans =\n\n{\n  ['file_1.dat']  ['file_2.dat']  ['file_3.dat']  ['file_4.dat']  ['file_5.dat']\n}\n",
            figure: None,
        },
    ),
    (
        "7e8d7d6f9e2f4c84942785337c620e4416e016ca26675053a65a2fe0f78bc487",
        Fragment {
            transcript: "--> strrep('Matlab is great','Matlab','FreeMat')\n\nans =\n\nFreeMat is great\n\n--> strrep({'time is money';'A stitch in time';'No time for games'},'time','money')\n\nans =\n",
            figure: None,
        },
    ),
    (
        "7f534b560d44c8f0924bd057d152c28658cfdb7dfa314dbc1b833c13d13f7f9d",
        Fragment {
            transcript: "--> y = @sin\n\ny =\n\n@sin\n\n--> x = func2str(y)\n\nx =\n\nsin\n\n--> y = @(x) x.^2\n\ny =\n\n@(x) x.^2\n\n--> x = func2str(y)\n\nx =\n\n@(x) x.^2\n",
            figure: None,
        },
    ),
    (
        "8188b83a3cab09eacedef393d4eaa2f760b64ad77d6b06fdbc63d21d79ffade0",
        Fragment {
            transcript: "--> cell(2,3,2)\n\nans =\n\n{\n  [0x0 double]  [0x0 double]  [0x0 double]\n  [0x0 double]  [0x0 double]  [0x0 double]\n}\n\n--> cell(1,3)\n\nans =\n\n{\n  [0x0 double]  [0x0 double]  [0x0 double]\n}\n\n--> cell([2,6])\n\nans =\n\n{\n  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]\n  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]  [0x0 double]\n}\n\n--> cell([1,3])\n\nans =\n\n{\n  [0x0 double]  [0x0 double]  [0x0 double]\n}\n",
            figure: None,
        },
    ),
    (
        "81f56e74fba21914f96c5efa9d38c2b9d9fdaf7542540c225d1a1c36ddc9ca89",
        Fragment {
            transcript: "--> ceil(3)\n\nans =\n\n3\n\n--> ceil(-3)\n\nans =\n\n-3\n\n--> ceil(float(3.023))\n\nans =\n\n4\n\n--> ceil(float(-2.341))\n\nans =\n\n-2\n\n--> ceil(4.312)\n\nans =\n\n5\n\n--> ceil(-5.32)\n\nans =\n\n-5\n",
            figure: None,
        },
    ),
    (
        "8246638c6f56ea859cf70fe4db6468abd1fd66bc81dad45733dc5d8684e6215c",
        Fragment {
            transcript: "--> A = [1,0,0;1,0,0;0,0,1]\n\nA =\n\n   1   0   0\n   1   0   0\n   0   0   1\n\n--> all(A)\n\nans =\n\n   0   0   0\n\n--> all(A>=0)\n\nans =\n\n   1   1   1\n\n--> all(A,2)\n\nans =\n\n   0\n   0\n   0\n",
            figure: None,
        },
    ),
    (
        "84fbbcc959aff9a4eb6eb2c5896ec23cd575def2631d6b7d708d4dfeb54cb212",
        Fragment {
            transcript: "--> pi\n\nans =\n\n3.1416\n\n--> cos(pi)\n\nans =\n\n-1\n",
            figure: None,
        },
    ),
    (
        "8633396defe202d48996fd40f4f2a5026a76865bede2a7257f695ecc6ab60229",
        Fragment {
            transcript: "--> bin2dec('101110')\n\nans =\n\n46\n\n--> bin2dec('010')\n\nans =\n\n2\n",
            figure: None,
        },
    ),
    (
        "88124e05f8a9137f7f0ccd82d7d9f31a6fba96a358a262e0c84f50a44fb28663",
        Fragment {
            transcript: "--> e\n\nans =\n\n2.7183\n\n--> log(e)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "88f80d362b5ecc2cfd3737d39dc68af93045804bb3496dab11b820f53b7a2606",
        Fragment {
            transcript: "--> A = diag([1.02f,3.04f,1.53f])\n\nA =\n\n   1.0200        0        0\n        0   3.0400        0\n        0        0   1.5300\n\n--> eig(A)\n\nans =\n\n   1.0200\n   3.0400\n   1.5300\n\n--> A = [1.0f,3.0f,4.0f;0,2.0f,6.7f;0.0f,0.0f,1.0f]\n\nA =\n\n        1        3        4\n        0        2   6.7000\n        0        0        1\n\n--> eig(A)\n\nans =\n\n   1\n   2\n   1\n\n--> A = float(randn(2))\n\nA =\n\n    0.1710    0.4110\n   -0.3091    0.4638\n\n--> [V,D] = eig(A)\n\nV =\n\n   0.8150 - 0.3656i   0.8150 + 0.3656i\n   0.5794 + 0.5143i   0.5794 - 0.5143i\n\n\nD =\n\n   0.3174 + 0.3250i             0 + 0i\n             0 + 0i   0.3174 - 0.3250i\n\n--> A*V - V*D\n\nans =\n\n   -5.5511e-17 - 2.7756e-17i   -5.5511e-17 + 2.7756e-17i\n                      0 + 0i                      0 + 0i\n\n--> B = [3,-2,-.9,2*eps;-2,4,1,-eps;-eps/4,eps/2,-1,0;-.5,-.5,.1,1]\n\nB =\n\n             3            -2       -0.9000    4.4409e-16\n            -2             4             1   -2.2204e-16\n   -5.5511e-17    1.1102e-16            -1             0\n       -0.5000       -0.5000        0.1000             1\n\n--> [VB,DB] = eig(B)\n\nVB =\n\n        0.6153       -0.4177    1.1102e-15       -0.1562\n       -0.7881       -0.3261    5.5511e-16        0.1375\n   -2.0463e-17   -2.9796e-18   -1.2326e-17            -1\n        0.0189        0.8482       -1.8875        0.0453\n\n\nDB =\n\n   5.5616        0        0        0\n        0   1.4384        0        0\n        0        0   1.0000        0\n        0        0        0       -1\n\n--> B*VB - VB*DB\n\nans =\n\n    2.6645e-15   -2.2204e-16    2.8312e-16    4.7184e-16\n    8.8818e-16    9.4369e-16   -1.4834e-16   -1.6653e-16\n    1.2619e-17   -5.7544e-18    2.4653e-17             0\n    2.0817e-16   -4.4409e-16    4.4409e-16    1.3878e-16\n\n--> [VN,DN] = eig(B,'nobalance')\n\nVN =\n\n        0.6153       -0.4177    1.1102e-15       -0.1562\n       -0.7881       -0.3261    5.5511e-16        0.1375\n   -2.0463e-17   -2.9796e-18   -1.2326e-17            -1\n        0.0189        0.8482       -1.8875        0.0453\n\n\nDN =\n\n   5.5616        0        0        0\n        0   1.4384        0        0\n        0        0   1.0000        0\n        0        0        0       -1\n\n--> B*VN - VN*DN\n\nans =\n\n    2.6645e-15   -2.2204e-16    2.8312e-16    4.7184e-16\n    8.8818e-16    9.4369e-16   -1.4834e-16   -1.6653e-16\n    1.2619e-17   -5.7544e-18    2.4653e-17             0\n    2.0817e-16   -4.4409e-16    4.4409e-16    1.3878e-16\n",
            figure: None,
        },
    ),
    (
        "8b110c55857d36a6613bcbeb95675fb0cb74283969d6e9b66921275beb8e55c2",
        Fragment {
            transcript: "--> hex2dec('3ff')\n\nans =\n\n1023\n\n--> hex2dec(['0ff';'2de';'123'])\n\nans =\n\n   255\n   734\n   291\n",
            figure: None,
        },
    ),
    (
        "8c12fe9e265456e26eb166b196bd1ea7db5c523e5478842fb81bd5c24b81e777",
        Fragment {
            transcript: "--> A = zeros(4)\n\nA =\n\n   0   0   0   0\n   0   0   0   0\n   0   0   0   0\n   0   0   0   0\n\n--> B = float(randn(2))\n\nB =\n\n    0.1710    0.4110\n   -0.3091    0.4638\n\n--> A(2:3,2:3) = B\n\nA =\n\n         0         0         0         0\n         0    0.1710    0.4110         0\n         0   -0.3091    0.4638         0\n         0         0         0         0\n\n--> C = A(2:3,1:end)\n\nC =\n\n         0    0.1710    0.4110         0\n         0   -0.3091    0.4638         0\n\n--> C = A(2:3,:)\n\nC =\n\n         0    0.1710    0.4110         0\n         0   -0.3091    0.4638         0\n\n--> D = zeros(2,2,3)\n\nD =\n\n(:,:,1) =\n\n   0   0\n   0   0\n\n(:,:,2) =\n\n   0   0\n   0   0\n\n(:,:,3) =\n\n   0   0\n   0   0\n\n--> D(:,:,2) = int32(10*rand(2,2))\n\nD =\n\n(:,:,1) =\n\n   0   0\n   0   0\n\n(:,:,2) =\n\n   2   0\n   3   7\n\n(:,:,3) =\n\n   0   0\n   0   0\n\n--> A = zeros(4)\n\nA =\n\n   0   0   0   0\n   0   0   0   0\n   0   0   0   0\n   0   0   0   0\n\n--> v = [1;2;3;4]\n\nv =\n\n   1\n   2\n   3\n   4\n\n--> A(2:3,2:3) = v\n\nA =\n\n   0   0   0   0\n   0   1   3   0\n   0   2   4   0\n   0   0   0   0\n\n--> A = {1, 'hello', [1:4]}\n\nA =\n\n{\n  [1]  ['hello']  [1x4 double]\n}\n\n--> A(1:2)\n\nans =\n\n{\n  [1]  ['hello']\n}\n\n--> A{1:2}\n\nans =\n\n1\n\n--> A = {[1,3,0],[5,2,7]}\n\nA =\n\n{\n  [1x3 double]  [1x3 double]\n}\n\n--> max(A{1:end})\n\nans =\n\n   5   3   7\n\n--> [K{1:2}] = max(randn(1,4))\n\nK =\n\n{\n  [1.2869]  [1]\n}\n\n--> C = [A{1};A{2}]\n\nC =\n\n   1   3   0\n   5   2   7\n\n--> clear A\n--> A.color = 'blue'\n\nA =\n\n1x1 struct array with fields:\n    color\n\n--> B = A.color\n\nB =\n\nblue\n\n--> clear A\n--> A(1).maxargs = [1,6,7,3]\n\nA =\n\n1x1 struct array with fields:\n    maxargs\n\n--> A(2).maxargs = [5,2,9,0]\n\nA =\n\n1x2 struct array with fields:\n    maxargs\n\n--> max(A.maxargs)\n\nans =\n\n   5   6   9   3\n\n--> clear A\n--> A(1).maxreturn = [];\n--> A(2).maxreturn = [];\n--> [A.maxreturn] = max(randn(1,4))\n\nA =\n\n1x2 struct array with fields:\n    maxreturn\n\n--> x.red = 430;\n--> x.green = 240;\n--> x.blue = 53;\n--> x.yello = 105\n\nx =\n\n1x1 struct array with fields:\n    red\n    green\n    blue\n    yello\n\n--> y = 'green'\n\ny =\n\ngreen\n\n--> a = x.(y)\n\na =\n\n240\n\n--> Z{3}.foo(2) = pi\n\nZ =\n\n{\n  [0x0 double]  [0x0 double]  [1x1 struct]\n}\n",
            figure: None,
        },
    ),
    (
        "8d98e1c6a35b00aea9e6d633ff501d781266cea1accd31cce386e66d54ae60d0",
        Fragment {
            transcript: "--> sin(.5)              % Calling the function directly\n\nans =\n\n0.4794\n\n--> y = str2func('sin')  % Convert it into a function handle\n\ny =\n\n@sin\n\n--> y(.5)                % Calling 'sin' via the function handle\n\nans =\n\n0.4794\n\n--> y = str2func('@(x) x.^2')\n\ny =\n\n@(x) x.^2\n\n--> y(2)\n\nans =\n\n4\n",
            figure: None,
        },
    ),
    (
        "931ace4312146d89ea51b51054d720e9acc7714658d428d3a3cc7b3db71bf3af",
        Fragment {
            transcript: "--> evenoddtest(4)\n4 is even\n--> evenoddtest(5)\n5 is odd\n--> evenoddtest(0)\n  \u{1b}[31m×\u{1b}[0m zero is neither even nor odd\n--> evenoddtest(pi)\n  \u{1b}[31m×\u{1b}[0m expecting integer argument\n",
            figure: None,
        },
    ),
    (
        "939cc76a98058d3bdbd7a772b75f3bce173659f9b9aa0c7aacfe4ab27931915e",
        Fragment {
            transcript: "--> x = rand(4,4,3);\n--> length(x)\n\nans =\n\n4\n\n--> numel(x)\n\nans =\n\n48\n\n--> numel(x,1:3,1:2,2)\n\nans =\n\n48\n",
            figure: None,
        },
    ),
    (
        "9535c8298ae4a3d9e143dc327d3e282aeaa47d2a0f4315739679efb498db4762",
        Fragment {
            transcript: "--> A = [1,2,3;4,5,6;7,8,0]\n\nA =\n\n   1   2   3\n   4   5   6\n   7   8   0\n\n--> p = poly(A)\n\np =\n\n          1         -6        -72   -27.0000\n\n--> r = roots(p)\n\nr =\n\n   12.1229\n   -5.7345\n   -0.3884\n",
            figure: None,
        },
    ),
    (
        "97d328136fe00980373897e229a1c6bcdea04c7a65a0c004f9875a9e82c4a4c5",
        Fragment {
            transcript: "--> a = randn(1,5)\n\na =\n\n    0.1710   -0.3091    0.4110    0.4638   -0.7223\n\n--> a>0\n\nans =\n\n   1   0   1   1   0\n\n--> a = [1,2,5,7,3]\n\na =\n\n   1   2   5   7   3\n\n--> b = [2,2,5,9,4]\n\nb =\n\n   2   2   5   9   4\n\n--> c = a == b\n\nc =\n\n   0   1   1   0   0\n",
            figure: None,
        },
    ),
    (
        "9a13f75255bd6596ed554d3d6acdfd9ba338be325bed80b7e8897b83e27cf2d4",
        Fragment {
            transcript: "--> strtrim('  lot of blank spaces    ');\n--> strtrim({'  space','enough ',' for ',''})\n\nans =\n",
            figure: None,
        },
    ),
    (
        "9aad8e6052e903a08bb98614b6ee0fe7f28efec0216aff5cd4e16d7ae044b9eb",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "9bc07ed70579beca823c7ebbf0c1373eb014f5ec308af6fec0bf83be561911e6",
        Fragment {
            transcript: "--> A = [1,1;0,1e-15]\n\nA =\n\n            1            1\n            0   1.0000e-15\n\n--> cond(A)\n\nans =\n\n2.0000e+15\n\n--> cond(A,1)\n\nans =\n\n2.0000e+15\n\n--> 1/rcond(A)\n\nans =\n\n2.0000e+15\n",
            figure: None,
        },
    ),
    (
        "9cf239edeeb573b2214c89904a9983c38bd793e9beb9164d6b04ccacae28ea60",
        Fragment {
            transcript: "--> cotd(45)\n\nans =\n\n1.0000\n",
            figure: None,
        },
    ),
    (
        "9d78c5190ad9935e0b4baec9cfd0b1bbe1548fee46761f56014670e92cdcf5ea",
        Fragment {
            transcript: "--> a = [1.2 3.4 inf 5]\n\na =\n\n   1.2000   3.4000      Inf        5\n\n--> isinf(a)\n\nans =\n\n   0   0   1   0\n\n--> b = 3./[2 5 0 3 1]\n\nb =\n\n   1.5000   0.6000      Inf        1        3\n",
            figure: None,
        },
    ),
    (
        "9f918be29f9e0dddca33f95ff8467f9393658ca5cd2a8362d4631f52254880e2",
        Fragment {
            transcript: "--> A = [1,2;4,5]\n\nA =\n\n   1   2\n   4   5\n\n--> permute(A,[2,1])\n\nans =\n\n   1   4\n   2   5\n\n--> A'\n\nans =\n\n   1   4\n   2   5\n\n--> A = randn(13,5,7,2);\n--> size(A)\n\nans =\n\n   13    5    7    2\n\n--> B = permute(A,[3,4,2,1]);\n--> size(B)\n\nans =\n\n    7    2    5   13\n",
            figure: None,
        },
    ),
    (
        "9fdc7c673ccb2fd88ebf11320eb46c6be3a32be04b530f5090723cc270fe8c84",
        Fragment {
            transcript: "--> A = randn(13,5,7,2);\n--> size(A)\n\nans =\n\n   13    5    7    2\n\n--> B = permute(A,[3,4,2,1]);\n--> size(B)\n\nans =\n\n    7    2    5   13\n\n--> C = ipermute(B,[3,4,2,1]);\n--> size(C)\n\nans =\n\n   13    5    7    2\n\n--> any(A~=C)\n\nans =\n\n(:,:,1,1) =\n\n   0   0   0   0   0\n\n(:,:,2,1) =\n\n   0   0   0   0   0\n\n(:,:,3,1) =\n\n   0   0   0   0   0\n\n(:,:,4,1) =\n\n   0   0   0   0   0\n\n(:,:,5,1) =\n\n   0   0   0   0   0\n\n(:,:,6,1) =\n\n   0   0   0   0   0\n\n(:,:,7,1) =\n\n   0   0   0   0   0\n\n(:,:,1,2) =\n\n   0   0   0   0   0\n\n(:,:,2,2) =\n\n   0   0   0   0   0\n\n(:,:,3,2) =\n\n   0   0   0   0   0\n\n(:,:,4,2) =\n\n   0   0   0   0   0\n\n(:,:,5,2) =\n\n   0   0   0   0   0\n\n(:,:,6,2) =\n\n   0   0   0   0   0\n\n(:,:,7,2) =\n\n   0   0   0   0   0\n",
            figure: None,
        },
    ),
    (
        "a2ed5441e43d97135d663eb95435ba4623a5d04c72847fcf3e692c1ec6649a2c",
        Fragment {
            transcript: "--> A = [5,1,3;3,2,1;0,3,1]\n\nA =\n\n   5   1   3\n   3   2   1\n   0   3   1\n\n--> min(A)\n\nans =\n\n   0   1   1\n\n--> min(A,[],2)\n\nans =\n\n   1\n   1\n   0\n\n--> min([5,3,2,9])\n\nans =\n\n2\n\n--> a = int8(100*randn(4))\n\na =\n\n     17    -72   -127     -1\n    -31    -32    -41     50\n     41     72      8   -113\n     46     48    127    -50\n\n--> b = int8(100*randn(4))\n\nb =\n\n     21     77     67     31\n    -25    106    -47    100\n   -124   -104     33     20\n    -69     21     57    -56\n\n--> min(a,b)\n\nans =\n\n     17    -72   -127     -1\n    -31    -32    -47     50\n   -124   -104      8   -113\n    -69     21     57    -56\n\n--> a = randn(2)\n\na =\n\n    0.1216    2.1308\n    0.1298   -0.2004\n\n--> min(a,0)\n\nans =\n\n         0         0\n         0   -0.2004\n",
            figure: None,
        },
    ),
    (
        "a7c1670964bc5bce686334891ebc028c40a0b125722ae6ebb0a441789dafb6b5",
        Fragment {
            transcript: "--> tand(45)\n\nans =\n\n1.0000\n",
            figure: None,
        },
    ),
    (
        "a7f822c8123d45154325c38668a9eb6a38df765c73d45969fb2dc2d8aabc9ad4",
        Fragment {
            transcript: "--> a = [1,0,0,5;0,3,2,0]\n\na =\n\n   1   0   0   5\n   0   3   2   0\n\n--> issparse(a)\n\nans =\n\n0\n\n--> A = sparse(a)\n\nA =\n\n  (1,1)      1\n  (2,2)      3\n  (2,3)      2\n  (1,4)      5\n\n--> issparse(A)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "ad0eae3a6803d0e8385f982b96c0e783ef79c5a3810a5eb35e3d6111aa209c5c",
        Fragment {
            transcript: "--> conj(3+4*i)\n\nans =\n\n3 - 4i\n\n--> conj([2,3,4])\n\nans =\n\n   2   3   4\n\n--> conj([2.0+3.0*i,i])\n\nans =\n\n   2 - 3i   0 - 1i\n",
            figure: None,
        },
    ),
    (
        "b1e94f0775966ded4d7d2a993155b8b01ddc048067b8032271cc960f34eabb1f",
        Fragment {
            transcript: "--> a = 32;\n--> b = 1:4;\n--> disp(a,b,pi)\n32\n",
            figure: None,
        },
    ),
    (
        "b21984f160166c1481d593b9d302d0db109f4fe8cd171b455fd14560a599ff85",
        Fragment {
            transcript: "--> a = sparse(rand(9));\n--> eigs(a)\n\nans =\n\n         4.3192 + 0i\n         0.9484 + 0i\n        -0.8602 + 0i\n    0.5781 + 0.4482i\n    0.5781 - 0.4482i\n   -0.5937 + 0.2283i\n\n--> eig(full(a))\n\nans =\n\n         4.3192 + 0i\n         0.9484 + 0i\n    0.5781 + 0.4482i\n    0.5781 - 0.4482i\n        -0.8602 + 0i\n   -0.5937 + 0.2283i\n   -0.5937 - 0.2283i\n   -0.0603 + 0.1713i\n   -0.0603 - 0.1713i\n\n--> eigs(a,4,'sm')\n\nans =\n\n   -0.0603 - 0.1713i\n   -0.0603 + 0.1713i\n   -0.5937 - 0.2283i\n   -0.5937 + 0.2283i\n\n--> eigs(a,4,'lr')\n\nans =\n\n        4.3192 + 0i\n        0.9484 + 0i\n   0.5781 + 0.4482i\n   0.5781 - 0.4482i\n\n--> eigs(a,4,'sr')\n\nans =\n\n        -0.8602 + 0i\n   -0.5937 + 0.2283i\n   -0.5937 - 0.2283i\n   -0.0603 + 0.1713i\n",
            figure: None,
        },
    ),
    (
        "b3a96902a6ff8b62ea739ac274065cd2165f161b7be3fcd5ccee588d4975ef4e",
        Fragment {
            transcript: "--> y = struct('foo',{1,3,4},'bar',{'cheese','cola','beer'},'key',508)\n\ny =\n\n1x3 struct array with fields:\n    foo\n    bar\n    key\n\n--> y(1)\n\nans =\n\n1x1 struct array with fields:\n    foo\n    bar\n    key\n\n--> y(2)\n\nans =\n\n1x1 struct array with fields:\n    foo\n    bar\n    key\n\n--> y(3)\n\nans =\n\n1x1 struct array with fields:\n    foo\n    bar\n    key\n\n--> Test(2,3).Type = 'Beer';\n--> Test(2,3).Ounces = 12;\n--> Test(2,3).Container = 'Can';\n--> Test(2,3)\n\nans =\n\n1x1 struct array with fields:\n    Type\n    Ounces\n    Container\n\n--> Test(1,1)\n\nans =\n\n1x1 struct array with fields:\n    Type\n    Ounces\n    Container\n",
            figure: None,
        },
    ),
    (
        "b4ad97ce2242d983fbebe63739a81bd9ba5ca3787523123da9aee1ce9e61ecbd",
        Fragment {
            transcript: "--> upper('this Is Strange CAPitalizaTion')\n\nans =\n\nTHIS IS STRANGE CAPITALIZATION\n\n--> upper({'This','Is','Strange','CAPitalizaTion'})\n\nans =\n\n{\n  ['THIS']  ['IS']  ['STRANGE']  ['CAPITALIZATION']\n}\n",
            figure: None,
        },
    ),
    (
        "b9cbbf9a1bea90dcf66a27e69c8e8d59a1037d0b5988b5b1781e2d49643aeac5",
        Fragment {
            transcript: "--> feps\n\nans =\n\n1.1921e-07\n\n--> 1.0f+eps\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "bbf321725a09751141a42af80da4e2bb2a12b56b45d2d31af9c2877bab02c2d7",
        Fragment {
            transcript: "--> lower('this Is Strange CAPitalizaTion')\n\nans =\n\nthis is strange capitalization\n\n--> lower({'This','Is','Strange','CAPitalizaTion'})\n\nans =\n\n{\n  ['this']  ['is']  ['strange']  ['capitalization']\n}\n",
            figure: None,
        },
    ),
    (
        "c072bd60ec52e187d599c17348fee16bae037b66309d86c8b28271c4ee83382d",
        Fragment {
            transcript: "--> rad2deg(1) % one radian is about 57 degrees\n\nans =\n\n57.2958\n\n--> rad2deg(pi/4) % should be 45 degrees\n\nans =\n\n45\n\n--> rad2deg(2*pi) % Note that this is 360 not 0 degrees\n\nans =\n\n360\n",
            figure: None,
        },
    ),
    (
        "c24da491f56956575c6762c6fc2098c1ecab6db3c5c58541f054ebf1430e36c1",
        Fragment {
            transcript: "--> x = [1 2 3 4]\n\nx =\n\n   1   2   3   4\n\n--> y = repmat(x,[5,1])\n\ny =\n\n   1   2   3   4\n   1   2   3   4\n   1   2   3   4\n   1   2   3   4\n   1   2   3   4\n\n--> x = [1 2;3 4]\n\nx =\n\n   1   2\n   3   4\n\n--> y = repmat(x,[1,1,3])\n\ny =\n\n(:,:,1) =\n\n   1   2\n   3   4\n\n(:,:,2) =\n\n   1   2\n   3   4\n\n(:,:,3) =\n\n   1   2\n   3   4\n",
            figure: None,
        },
    ),
    (
        "c25ca3ffa89bfee257c4512fa7142ee60b2ddcf6d5045e82464056178a39d54b",
        Fragment {
            transcript: "--> A = [2;5;6;2]\n\nA =\n\n   2\n   5\n   6\n   2\n\n--> B = int2bin(A,8)\n\nB =\n\n   0   0   0   0   0   0   1   0\n   0   0   0   0   0   1   0   1\n   0   0   0   0   0   1   1   0\n   0   0   0   0   0   0   1   0\n\n--> bin2int(B)\n\nans =\n\n   2\n   5\n   6\n   2\n\n--> A = [1;2;-5;2]\n\nA =\n\n    1\n    2\n   -5\n    2\n\n--> B = int2bin(A,8)\n\nB =\n\n   0   0   0   0   0   0   0   1\n   0   0   0   0   0   0   1   0\n   1   1   1   1   1   0   1   1\n   0   0   0   0   0   0   1   0\n\n--> bin2int(B)\n\nans =\n\n     1\n     2\n   251\n     2\n\n--> int32(bin2int(B))\n\nans =\n\n     1\n     2\n   251\n     2\n",
            figure: None,
        },
    ),
    (
        "c43735668c971bcf27f998d9e3aa16932512573c5b6267dc8c38828255494288",
        Fragment {
            transcript: "--> a = [1.2 3.4 nan 5]\n\na =\n\n   1.2000   3.4000      NaN        5\n\n--> isnan(a)\n\nans =\n\n   0   0   1   0\n",
            figure: None,
        },
    ),
    (
        "c4d03e3c272b04eb9df3205be57d7bffb336138e1871d6952ada2414c7b25afa",
        Fragment {
            transcript: "--> a.foo = 32\n\na =\n\n1x1 struct array with fields:\n    foo\n\n--> a.goo = 64\n\na =\n\n1x1 struct array with fields:\n    foo\n    goo\n\n--> isfield(a,'goo')\n\nans =\n\n1\n\n--> isfield(a,'got')\n\nans =\n\n0\n\n--> isfield(pi,'round')\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "c57ccbcd3d11ec92f02a5b2a63699595656f6f1fe8743027f2b96b624e73c91b",
        Fragment {
            transcript: "--> a = []\n\na =\n\n[]\n\n--> isempty(a)\n\nans =\n\n1\n\n--> b = 1:3\n\nb =\n\n   1   2   3\n\n--> isempty(b)\n\nans =\n\n0\n\n--> clear x\n--> isempty(x)\n  \u{1b}[31m×\u{1b}[0m undefined variable or function 'x'\n   ╭────\n \u{1b}[2m1\u{1b}[0m │ isempty(x)\n   · \u{1b}[35;1m        ┬\u{1b}[0m\n   ·         \u{1b}[35;1m╰── \u{1b}[35;1mhere\u{1b}[0m\u{1b}[0m\n   ╰────\n",
            figure: None,
        },
    ),
    (
        "c627d5233fe62b20d40ec7c92b8c2ee2708726e86936cedc77689dadcc06b736",
        Fragment {
            transcript: "--> abs(3+4*i)\n\nans =\n\n5\n\n--> abs([-2,3,-4,5])\n\nans =\n\n   2   3   4   5\n\n--> abs([2.0+3.0*i,i])\n\nans =\n\n   3.6056        1\n",
            figure: None,
        },
    ),
    (
        "c7cdb7cce1393ac4c365fd546fbf0b72eeb77c1694f7a0d3c54a0886fba80162",
        Fragment {
            transcript: "--> ones(2,3,2)\n\nans =\n\n(:,:,1) =\n\n   1   1   1\n   1   1   1\n\n(:,:,2) =\n\n   1   1   1\n   1   1   1\n\n--> ones(1,3)\n\nans =\n\n   1   1   1\n\n--> ones([2,6])\n\nans =\n\n   1   1   1   1   1   1\n   1   1   1   1   1   1\n\n--> ones([1,3])\n\nans =\n\n   1   1   1\n\n--> uint16(ones(3))\n\nans =\n\n   1   1   1\n   1   1   1\n   1   1   1\n",
            figure: None,
        },
    ),
    (
        "c91427809dde8b41c5f6ba1770a4dde0f84755a7a544abbf6987e577a556776f",
        Fragment {
            transcript: "--> floor(3)\n\nans =\n\n3\n\n--> floor(-3)\n\nans =\n\n-3\n\n--> floor(3.023)\n\nans =\n\n3\n\n--> floor(-2.341)\n\nans =\n\n-3\n",
            figure: None,
        },
    ),
    (
        "caac7596c3194c68d6274650959f434b3c8c5ba6f7eef1036b08bac32974d591",
        Fragment {
            transcript: "--> set_global('Hello')\n--> get_global\n\nans =\n\nHello\n",
            figure: None,
        },
    ),
    (
        "caacfc16f9f4285d39fbadaad9fe01b37e66b171b519e74ad581703b4108553d",
        Fragment {
            transcript: "--> x = [1,2,3;5,6,7]\n\nx =\n\n   1   2   3\n   5   6   7\n\n--> csvwrite('csvwrite.csv',x)\n--> csvread('csvwrite.csv')\n\nans =\n\n   1   2   3\n   5   6   7\n\n--> csvwrite('csvwrite.csv',x,1,2)\n--> csvread('csvwrite.csv')\n\nans =\n\n   1   2   3\n   5   6   7\n",
            figure: None,
        },
    ),
    (
        "cb4b5cab85b2eb7997a7f2ceba4b8264444f66228a7806de9e027bd8f8d71e0b",
        Fragment {
            transcript: "--> break_ex\n\nans =\n\n15\n\n--> sum(1:5)\n\nans =\n\n15\n",
            figure: None,
        },
    ),
    (
        "cd3a19a2a201fdff4ff327340bcf38a54acc15d78d0e79a66ea20dee3f3535bf",
        Fragment {
            transcript: "--> roots([1 -6 -72 -27])\n\nans =\n\n   12.1229\n   -5.7345\n   -0.3884\n",
            figure: None,
        },
    ),
    (
        "cd6cc9dc602b8f4d1e8209613b9af0d0942f6e4fb75ef36427fc01542a608b9f",
        Fragment {
            transcript: "--> [X,Y] = meshgrid(-2:.4:2)\n\nX =\n\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n        -2   -1.6000   -1.2000   -0.8000   -0.4000         0    0.4000    0.8000    1.2000    1.6000         2\n\n\nY =\n\n        -2        -2        -2        -2        -2        -2        -2        -2        -2        -2        -2\n   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000   -1.6000\n   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000   -1.2000\n   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000   -0.8000\n   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000   -0.4000\n         0         0         0         0         0         0         0         0         0         0         0\n    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000    0.4000\n    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000    0.8000\n    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000    1.2000\n    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000    1.6000\n         2         2         2         2         2         2         2         2         2         2         2\n\n--> [X,Y] = meshgrid([1,2,3,4],[6,7,8])\n\nX =\n\n   1   2   3   4\n   1   2   3   4\n   1   2   3   4\n\n\nY =\n\n   6   6   6   6\n   7   7   7   7\n   8   8   8   8\n",
            figure: None,
        },
    ),
    (
        "cf2c0668463a47e07e760748ba4228810ba7855414d031f7450a2667d72a28fc",
        Fragment {
            transcript: "--> a = [1,0,3,0,5;0,0,2,3,0;1,0,0,0,1]\n\na =\n\n   1   0   3   0   5\n   0   0   2   3   0\n   1   0   0   0   1\n\n--> b = spones(a)\n\nb =\n\n  (1,1)      1\n  (3,1)      1\n  (1,3)      1\n  (2,3)      1\n  (2,4)      1\n  (1,5)      1\n  (3,5)      1\n\n--> full(b)\n\nans =\n\n   1   0   1   0   1\n   0   0   1   1   0\n   1   0   0   0   1\n",
            figure: None,
        },
    ),
    (
        "d007365394cbe405d9390d7907eb19f545c5684cf8480e4251e7f46d74be8212",
        Fragment {
            transcript: "--> A = [1,2,4,3;2,3,4,5]\n\nA =\n\n   1   2   4   3\n   2   3   4   5\n\n--> vec(A)\n\nans =\n\n   1\n   2\n   2\n   3\n   4\n   4\n   3\n   5\n",
            figure: None,
        },
    ),
    (
        "d0cad6fd3c0ba14b8b1727ebad581a3afb523b7c1370703c3911c1531fd895f5",
        Fragment {
            transcript: "--> [c1] = varoutfunc\n\nc1 =\n\none of one\n\n--> [c1,c2] = varoutfunc\n\nc1 =\n\none of two\n\n\nc2 =\n\ntwo of two\n\n--> [c1,c2,c3] = varoutfunc\n\nc1 =\n\none of three\n\n\nc2 =\n\ntwo of three\n\n\nc3 =\n\nthree of three\n",
            figure: None,
        },
    ),
    (
        "d18953934bb01ba497a12ef3d024c69f6bb5dbc96d6b7f54843401116a32d933",
        Fragment {
            transcript: "--> y.foo = 3; y.goo = 'hello';\n--> x = fieldnames(y)\n\nx =\n\n{\n  ['foo']\n  ['goo']\n}\n",
            figure: None,
        },
    ),
    (
        "d3a916932f9f048fbd8291726ca4640f2957e9b0c2535a2e6e55d98d6e3a74b9",
        Fragment {
            transcript: "--> rank([1,3,2;4,5,6])\n\nans =\n\n2\n\n--> rank([1,2,3;2,4,6])\n\nans =\n\n1\n\n--> A = [1,0;0,eps/2]\n\nA =\n\n            1            0\n            0   1.1102e-16\n\n--> rank(A)\n\nans =\n\n1\n\n--> rank(A,eps/8)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "d4766f634c299f5b15990e976f3aae559a1b8a23bbad6e33a14b8c48aaf4b2dd",
        Fragment {
            transcript: "--> deg2rad(360) - 2*pi\n\nans =\n\n0\n",
            figure: None,
        },
    ),
    (
        "d487b5a8d3ae5cb702a6827615cb026c66dcf78beac5d15c2dcf21afbc7dca40",
        Fragment {
            transcript: "--> bitcmp(uint16(2^14-2))\n\nans =\n\n49153\n\n--> bitcmp(uint16(2^14-2),14)\n\nans =\n\n1\n",
            figure: None,
        },
    ),
    (
        "d61e3fe06955bf6074cc7740ff1b5ff88803cb0979edf750d79fb912d522f17a",
        Fragment {
            transcript: "--> self(4)\ny is initialized to one\ny is 3\ny is 6\ny is 10\n\nans =\n\n10\n\n--> self(6)\ny is initialized to one\ny is 3\ny is 6\ny is 10\ny is 15\ny is 21\n\nans =\n\n21\n\n--> retall\n",
            figure: None,
        },
    ),
    (
        "d700751baa1e7eba4431f8575c8b42369b3fe606e9f01cbcd0255685888c4f74",
        Fragment {
            transcript: "--> csvread('sample_data.csv')\n\nans =\n\n   10   12   13    0   45   16\n    9   11   52   93    5    6\n    1    3    4    4   90   -3\n   14   17   13   67   30   43\n   21   33   14   44    1    0\n\n--> csvread('sample_data.csv',1,2)\n\nans =\n\n   52   93    5    6\n    4    4   90   -3\n   13   67   30   43\n   14   44    1    0\n\n--> csvread('sample_data.csv',1,2,[1,2,3,4])\n\nans =\n\n   52   93    5    6\n    4    4   90   -3\n   13   67   30   43\n   14   44    1    0\n",
            figure: None,
        },
    ),
    (
        "d8529f364497bb791bc422181f1e6778e8a3dd0a1719230ad4c35aec126cf46e",
        Fragment {
            transcript: "--> logspace(1,2,3)\n\nans =\n\n        10   31.6228       100\n",
            figure: None,
        },
    ),
    (
        "da7417241572d94e56d29fac2c9cb27eda475f27950b7d83dfc54052d6c45e3e",
        Fragment {
            transcript: "--> x = 3+4*i\n\nx =\n\n3 + 4i\n\n--> a = abs(x)\n\na =\n\n5\n\n--> t = angle(x)\n\nt =\n\n0.9273\n\n--> a*exp(i*t)\n\nans =\n\n3.0000 + 4.0000i\n",
            figure: None,
        },
    ),
    (
        "db8ee98c097494c1b5c4228f7081d481b26ccb6975b22bb5d26b4b02c23e77bb",
        Fragment {
            transcript: "--> round(3)\n\nans =\n\n3\n\n--> round(-3)\n\nans =\n\n-3\n\n--> round(3.023f)\n\nans =\n\n3\n\n--> round(-2.341f)\n\nans =\n\n-2\n\n--> round(4.312)\n\nans =\n\n4\n\n--> round(-5.32)\n\nans =\n\n-5\n",
            figure: None,
        },
    ),
    (
        "dbe94d9108bc2761ad8fb6b01ff8d220d57e326812fbb99a8f0b71252bad5c71",
        Fragment {
            transcript: "--> a = rand(8); a(a>0.2) = 0;\n--> A = sparse(a)\n\nA =\n\n  (7,1)      0.0301\n  (4,2)      0.0997\n  (7,2)      0.1200\n  (8,2)      0.0056\n  (5,3)      0.0194\n  (6,3)      0.1227\n  (8,3)      0.0219\n  (1,5)      0.0342\n  (1,6)      0.0837\n  (6,6)      0.0915\n  (8,6)      0.1055\n  (2,8)      0.0077\n\n--> nonzeros(A)\n\nans =\n\n   0.0301\n   0.0997\n   0.1200\n   0.0056\n   0.0194\n   0.1227\n   0.0219\n   0.0342\n   0.0837\n   0.0915\n   0.1055\n   0.0077\n",
            figure: None,
        },
    ),
    (
        "dc5dd150a33b028a28262283cbb0c6b66b0bd55ef841c0bf0f8d6f9d0b90db5e",
        Fragment {
            transcript: "--> a = [-1.8,pi,8,-pi,-0.001,2.3+0.3i]\n\na =\n\n       -1.8000 + 0i        3.1416 + 0i             8 + 0i       -3.1416 + 0i       -0.0010 + 0i   2.3000 + 0.3000i\n\n--> fix(a)\n\nans =\n\n   -1    3    8   -3    0    2\n",
            figure: None,
        },
    ),
    (
        "dfc4959d2e9fae583da150811ebeba7608d798233a302fa60b559b69628a6a59",
        Fragment {
            transcript: "--> inf*0\n\nans =\n\nNaN\n\n--> inf*2\n\nans =\n\nInf\n\n--> inf*-2\n\nans =\n\n-Inf\n\n--> inf/inf\n\nans =\n\nNaN\n\n--> inf/0\n\nans =\n\nInf\n\n--> inf/nan\n\nans =\n\nNaN\n\n--> uint32(inf)\n\nans =\n\n4294967295\n\n--> complex(inf)\n\nans =\n\nInf + 0i\n",
            figure: None,
        },
    ),
    (
        "e28a182a7b91488b3ac39762cd6c42d6acea6889ee3a8a8d6265adf5c39289bf",
        Fragment {
            transcript: "--> a = 2; b = 4;    % define a and b (slope and intercept)\n--> y = @(x) a*x+b   % create the anonymous function\n\ny =\n\n@(x) a*x+b\n\n--> y(2)             % evaluate it for x = 2\n\nans =\n\n8\n\n--> a = 5; b = 7;    % change a and b\n--> y(2)             % the value did not change!  because a=2,b=4 are captured in y\n\nans =\n\n8\n\n--> y = @(x) a*x+b   % recreate the function\n\ny =\n\n@(x) a*x+b\n\n--> y(2)             % now the new values are used\n\nans =\n\n17\n",
            figure: None,
        },
    ),
    (
        "e7b0ec34a45227be6372663bf97e71e84a336b496c7a218d6fc61f69b5384d34",
        Fragment {
            transcript: "--> asecd(2/sqrt(2))\n\nans =\n\n45\n\n--> asecd(2)\n\nans =\n\n60.0000\n",
            figure: None,
        },
    ),
    (
        "e7de2fee23785dc47802404cd941e1bd09a846553644e805192d539eb0d75e3b",
        Fragment {
            transcript: "--> strcattest('hi','ho')\nstr1 = hi, str2 = ho, str3 = hiho\n--> strcattest 'hi' 'ho'\nstr1 = hi, str2 = ho, str3 = hiho\n--> strcattest hi ho\nstr1 = hi, str2 = ho, str3 = hiho\n",
            figure: None,
        },
    ),
    (
        "e84e56ce1031d2d15223f4b9ea57abf403fedab0f01d3a1adfd6e5b05672f188",
        Fragment {
            transcript: "--> sqrt(9)\n\nans =\n\n3\n\n--> sqrt(i)\n\nans =\n\n0.7071 + 0.7071i\n\n--> sqrt(-1)\n\nans =\n\n0 + 1i\n\n--> x = rand(4)\n\nx =\n\n   0.6037   0.2148   0.8515   0.4986\n   0.4148   0.3106   0.3519   0.6329\n   0.6727   0.0301   0.5200   0.1200\n   0.7656   0.6528   0.0997   0.0056\n\n--> sqrt(x)\n\nans =\n\n   0.7770   0.4635   0.9228   0.7061\n   0.6441   0.5573   0.5932   0.7956\n   0.8202   0.1735   0.7211   0.3465\n   0.8750   0.8079   0.3158   0.0751\n",
            figure: None,
        },
    ),
    (
        "ebf91feb707436cc86772c4718ab49696331333b1992df89a932f4b50dd3929d",
        Fragment {
            transcript: "--> continue_ex\n\nans =\n\n9\n\n--> sum([1:4,6:10])\n\nans =\n\n50\n",
            figure: None,
        },
    ),
    (
        "eca55cd4eace0d65e8c8cc899ad5cb6007ede4bf8f3e5aa5a145e2be23d85179",
        Fragment {
            transcript: "--> asind(sqrt(2)/2)\n\nans =\n\n45.0000\n\n--> asind(0.5)\n\nans =\n\n30.0000\n",
            figure: None,
        },
    ),
    (
        "ede914bf547a179748d117c8fe3e314b3cf72526644e18034a458b1c17cde530",
        Fragment {
            transcript: "--> real(3+4*i)\n\nans =\n\n3\n\n--> real([2,3,4])\n\nans =\n\n   2   3   4\n\n--> real([2.0+3.0*i,i])\n\nans =\n\n   2   0\n",
            figure: None,
        },
    ),
    (
        "f0a61ce3c38b22a79dfe46ba58b6a552c2964d1d0445a848a0979a246b3cdacb",
        Fragment {
            transcript: "",
            figure: None,
        },
    ),
    (
        "f55dc9e7756a04edef87aab5eb45e4508939f1951ba5253114803c4efa6d281b",
        Fragment {
            transcript: "--> deblank('hello   ')\n\nans =\n\nhello\n\n--> deblank({'hello  ','there ','  is  ','  sign  '})\n\nans =\n",
            figure: None,
        },
    ),
    (
        "f7031e71506ccb04ced050f9d6902e2bd41d5ba4eb9d935a633157656603ba7c",
        Fragment {
            transcript: "--> a = [1,0,0,5;0,3,2,0]\n\na =\n\n   1   0   0   5\n   0   3   2   0\n\n--> nnz(a)\n\nans =\n\n4\n\n--> A = sparse(a)\n\nA =\n\n  (1,1)      1\n  (2,2)      3\n  (2,3)      2\n  (1,4)      5\n\n--> nnz(A)\n\nans =\n\n4\n",
            figure: None,
        },
    ),
    (
        "f7854caa7ffaada8d4281a8494c1cb8b4ade16352e3c0f42fa5f225fa70666e2",
        Fragment {
            transcript: "--> a = [1,2,5,2,4];\n--> find(a==2)\n\nans =\n\n   2   4\n\n--> A = [1,0,3;0,2,1;3,0,0]\n\nA =\n\n   1   0   3\n   0   2   1\n   3   0   0\n\n--> n = find(A==0)\n\nn =\n\n   2\n   4\n   6\n   9\n\n--> A(n) = 5\n\nA =\n\n   1   5   3\n   5   2   1\n   3   5   5\n\n--> A = [1,0,3;0,2,1;3,0,0]\n\nA =\n\n   1   0   3\n   0   2   1\n   3   0   0\n\n--> A(A==0) = 5\n\nA =\n\n   1   5   3\n   5   2   1\n   3   5   5\n\n--> A = [1,0,3;0,2,1;3,0,0]\n\nA =\n\n   1   0   3\n   0   2   1\n   3   0   0\n\n--> [r,c] = find(A)\n\nr =\n\n   1\n   3\n   2\n   1\n   2\n\n\nc =\n\n   1\n   1\n   2\n   3\n   3\n\n--> [r,c,v] = find(A)\n\nr =\n\n   1\n   3\n   2\n   1\n   2\n\n\nc =\n\n   1\n   1\n   2\n   3\n   3\n\n\nv =\n\n   1\n   3\n   2\n   3\n   1\n",
            figure: None,
        },
    ),
    (
        "f9c4f2997e524cf54a3acf2705414d1799fb94bbd880b5e5c6e8402550337769",
        Fragment {
            transcript: "--> sprintf(['x0123456789y\\n','x',blanks(10),'y\\n'])\n\nans =\n\nx0123456789y\nx          y\n",
            figure: None,
        },
    ),
    (
        "faeec5640a85e632dbefd1bfa2e82f542d40579ce28527bc71408ad2ef0d55ef",
        Fragment {
            transcript: "--> a = randn(23,12,5);\n--> size(a)\n\nans =\n\n   23   12    5\n\n--> size(a,2)\n\nans =\n\n12\n",
            figure: None,
        },
    ),
    (
        "fc86b197487449b79dcd4685796ed5a6d874f94ebf96cabc0f59a944b67d9185",
        Fragment {
            transcript: "--> a = (randn(1,6)>0)\n\na =\n\n   1   0   1   1   0   0\n\n--> b = (randn(1,6)>0)\n\nb =\n\n   0   1   1   0   1   0\n\n--> c = a | b\n\nc =\n\n   1   1   1   1   1   0\n\n--> d = a & b\n\nd =\n\n   0   0   1   0   0   0\n\n--> xor = c & (~d)\n\nxor =\n\n   1   1   0   1   1   0\n\n--> c(d) = 0\n\nc =\n\n   1   1   0   1   1   0\n",
            figure: None,
        },
    ),
    (
        "fccaecdf0ac117fa436713f63db0795b21cd8beb3a69371c31369045785e0ce0",
        Fragment {
            transcript: "--> i\n\nans =\n\n0 + 1i\n\n--> i^2\n\nans =\n\n-1 + 1.2246e-16i\n\n--> j\n\nans =\n\n0 + 1i\n\n--> j^2\n\nans =\n\n-1 + 1.2246e-16i\n\n--> accum = 0; for i=1:100; accum = accum + i; end; accum\n\nans =\n\n5050\n\n--> i\n\nans =\n\n100\n\n--> clear i\n--> i\n\nans =\n\n0 + 1i\n",
            figure: None,
        },
    ),
    (
        "fe9b1c4386c15eb30f59e6e98301f8499016ec1702466ea9e3f3a5de46defaa1",
        Fragment {
            transcript: "--> A = float(rand(1,2))\n\nA =\n\n   0.6037   0.4148\n\n--> B = pinv(A)\n\nB =\n\n   1.1252\n   0.7731\n\n--> A*B*A\n\nans =\n\n   0.6037   0.4148\n\n--> B*A*B\n\nans =\n\n   1.1252\n   0.7731\n\n--> A*B\n\nans =\n\n1.0000\n\n--> B*A\n\nans =\n\n   0.6793   0.4667\n   0.4667   0.3207\n\n--> A = float([1;1;1;1])\n\nA =\n\n   1\n   1\n   1\n   1\n\n--> pinv(A)\n\nans =\n\n   0.2500   0.2500   0.2500   0.2500\n\n--> A = float([1,1])\n\nA =\n\n   1   1\n\n--> pinv(A) * 5.0\n\nans =\n\n   2.5000\n   2.5000\n",
            figure: None,
        },
    ),
];

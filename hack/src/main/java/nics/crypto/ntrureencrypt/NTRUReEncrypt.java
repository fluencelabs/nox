package nics.crypto.ntrureencrypt;

import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Random;
import net.sf.ntru.encrypt.EncryptionKeyPair;
import net.sf.ntru.encrypt.EncryptionParameters;
import net.sf.ntru.encrypt.EncryptionPrivateKey;
import net.sf.ntru.encrypt.EncryptionPublicKey;
import net.sf.ntru.encrypt.NtruEncrypt;
import net.sf.ntru.polynomial.IntegerPolynomial;
import net.sf.ntru.polynomial.Polynomial;

/**
 *
 * @author David Nuñez <dnunez (at) lcc.uma.es>
 */
public class NTRUReEncrypt {


    EncryptionParameters params;
    NtruEncrypt ntru;

    private IntegerPolynomial one;

    static boolean out = false;

    /**
     * Constructs a new instance with a set of encryption parameters.
     *
     * @param params encryption parameters
     */
    public NTRUReEncrypt(EncryptionParameters params) {
        ntru = new NtruEncrypt(params);
        this.params = params;

        one = new IntegerPolynomial(params.N);
        one.coeffs[0] = 1;
    }

    private Polynomial generateBlindingPolynomial(byte[] seed) throws Exception {

//        for(Method m : ntru.getClass().getDeclaredMethods()){
//            System.out.println(m.getName());
//        }
        Method m = ntru.getClass().getDeclaredMethod("generateBlindingPoly", byte[].class);
        m.setAccessible(true);
        return (Polynomial) m.invoke(ntru, seed);
    }

    private static IntegerPolynomial extractH(EncryptionPublicKey pub) throws Exception {
        Field f = EncryptionPublicKey.class.getDeclaredField("h");
        f.setAccessible(true);
        return ((IntegerPolynomial) f.get(pub)).toIntegerPolynomial();
    }

    public EncryptionKeyPair generateKeyPair() {
        return this.ntru.generateKeyPair();
    }



    public ReEncryptionKey generateReEncryptionKey(EncryptionPrivateKey pA, EncryptionPrivateKey pB) throws Exception {

        IntegerPolynomial fA = privatePolynomial(pA);
        IntegerPolynomial fB = privatePolynomial(pB);

        return new ReEncryptionKey(fA, fB, params.q);
    }

    public IntegerPolynomial encrypt(EncryptionPublicKey pub, IntegerPolynomial m) throws Exception {

        Polynomial r = generateBlindingPolynomial(generateSeed()); // oid es cualquier cosa

        out("r = " + Arrays.toString(r.toIntegerPolynomial().coeffs));

        IntegerPolynomial h = extractH(pub);

        out("h = " + Arrays.toString(h.coeffs));

        IntegerPolynomial e = r.mult(h);

        out("e = " + Arrays.toString(e.coeffs));

        e.add(m);

        out("e = " + Arrays.toString(e.coeffs));

        e.ensurePositive(params.q);

        out("e = " + Arrays.toString(e.coeffs));

        return e;

    }

    public IntegerPolynomial reEncrypt(ReEncryptionKey rk, IntegerPolynomial c) throws Exception {

        Polynomial r = generateBlindingPolynomial(generateSeed()); // oid es cualquier cosa



        IntegerPolynomial ruido = r.toIntegerPolynomial();
        ruido.mult(3);
        ruido.modCenter(params.q);


        IntegerPolynomial c_prime = c.mult(rk.rk);
        c_prime.add(ruido);
        c_prime.ensurePositive(params.q);

        return c_prime;

    }

    public IntegerPolynomial decrypt(EncryptionPrivateKey priv, IntegerPolynomial c) throws Exception {

        IntegerPolynomial f = privatePolynomial(priv);

        IntegerPolynomial a = c.toIntegerPolynomial().mult(f);

        a.modCenter(params.q);

        IntegerPolynomial m = a.toIntegerPolynomial();

        m.mod3();

        return m;

    }

    private static void out(String s) {
        if (out) {
            System.out.println(s);
        }
    }

    public IntegerPolynomial message(byte[] msg) {
        // Crea un mensaje aleatorio con dm 0's, dm 1's y dm -1's.
        IntegerPolynomial m = new IntegerPolynomial(params.N);

        Random rand = new SecureRandom(msg);
        ArrayList<Integer> list = new ArrayList<Integer>();

        int dm0 = 106; // params.dm0; FIXED FROM ees1171ep1

        while (list.size() < dm0 * 3) {
            Integer i = rand.nextInt(params.N);
            if (!list.contains(i)) {
                list.add(i);
            }
        }
        for (int j = 0; j < dm0; j++) {
            m.coeffs[list.get(j)] = 0;
        }
        for (int j = dm0; j < 2 * dm0; j++) {
            m.coeffs[list.get(j)] = -1;
        }
        for (int j = 2 * dm0; j < 3 * dm0; j++) {
            m.coeffs[list.get(j)] = 1;
        }
        out("m = " + Arrays.toString(m.coeffs));
        return m;
    }

    public static IntegerPolynomial extractF(EncryptionPrivateKey priv) throws Exception {
        Field f = EncryptionPrivateKey.class.getDeclaredField("t");
        f.setAccessible(true);
        return ((Polynomial) f.get(priv)).toIntegerPolynomial();
    }

    public IntegerPolynomial privatePolynomial(EncryptionPrivateKey priv) throws Exception {

        IntegerPolynomial f = extractF(priv);
        f.mult(3);
        f.add(one);

        return f;
    }

    private byte[] generateSeed() {
        return new byte[]{1, 2, 3, 4};
    }
}
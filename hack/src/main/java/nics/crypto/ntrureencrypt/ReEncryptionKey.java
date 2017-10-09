package nics.crypto.ntrureencrypt;

import net.sf.ntru.polynomial.IntegerPolynomial;

/**
 * @author David Nuñez <dnunez (at) lcc.uma.es>
 */
public class ReEncryptionKey {

    public IntegerPolynomial rk;

    public ReEncryptionKey(IntegerPolynomial fA, IntegerPolynomial fB, int q) {

        IntegerPolynomial fBinv = fB.toIntegerPolynomial().invertFq(q);

        rk = fA.toIntegerPolynomial().mult(fBinv);
    }

}
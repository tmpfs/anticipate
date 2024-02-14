#!../programs/read-password.sh

#$ expect Password:
foo-pass
#$ expect Confirm password:
bar-pass
#$ regex Error
